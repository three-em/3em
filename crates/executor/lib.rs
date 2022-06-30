pub mod executor;
pub mod test_util;
pub mod utils;

use crate::executor::raw_execute_contract;
pub use crate::executor::ExecuteResult;
pub use crate::executor::ValidityTable;
use deno_core::error::{generic_error, AnyError};
use deno_core::serde_json::Value;
pub use indexmap::map::IndexMap;
use lru::LruCache;
use once_cell::sync::Lazy;
use std::cmp::Ordering;
use std::ffi::CString;
use std::sync::Mutex;
use three_em_arweave::arweave::get_cache;
use three_em_arweave::arweave::Arweave;
use three_em_arweave::arweave::LoadedContract;
use three_em_arweave::gql_result::GQLEdgeInterface;
use three_em_arweave::gql_result::GQLNodeInterface;
use three_em_arweave::miscellaneous::get_sort_key;
use three_em_evm::Instruction;
use three_em_evm::U256;

static LRU_CACHE: Lazy<Mutex<LruCache<String, ExecuteResult>>> =
  Lazy::new(|| Mutex::new(LruCache::unbounded()));

pub async fn simulate_contract(
  contract_id: String,
  contract_init_state: Option<String>,
  interactions: Vec<GQLEdgeInterface>,
  arweave: &Arweave,
  maybe_cache: Option<bool>
) -> Result<ExecuteResult, AnyError> {
  let shared_id = contract_id.clone();
  let loaded_contract = tokio::join!(async move {
    let contract: Result<LoadedContract, AnyError> = arweave
      .load_contract(shared_id, None, None, contract_init_state, maybe_cache.unwrap_or(false), true)
      .await;

    contract
  })
  .0;

  if loaded_contract.is_ok() {
    let execute = raw_execute_contract(
      contract_id,
      loaded_contract.unwrap(),
      interactions,
      IndexMap::new(),
      None,
      true,
      false,
      |validity_table, cache_state| {
        ExecuteResult::V8(cache_state.unwrap(), validity_table)
      },
      arweave,
    )
    .await;

    Ok(execute)
  } else {
    Err(generic_error("Contract could not be loaded"))
  }
}

#[async_recursion::async_recursion(?Send)]
pub async fn execute_contract(
  contract_id: String,
  height: Option<usize>,
  cache: bool,
  show_errors: bool,
  contract_src_tx: Option<String>,
  contract_content_type: Option<String>,
  arweave: &Arweave,
) -> Result<ExecuteResult, AnyError> {
  if let Some(result) = LRU_CACHE.lock().unwrap().get(&contract_id) {
    return Ok(result.clone());
  }

  let contract_id_copy = contract_id.to_owned();
  let contract_id_copy2 = contract_id.to_owned();
  let shared_id = contract_id.clone();
  let (loaded_contract, interactions) = tokio::join!(
    async move {
      let contract: Result<LoadedContract, AnyError> = arweave
        .load_contract(
          shared_id,
          contract_src_tx,
          contract_content_type,
          None,
          cache,
          false,
        )
        .await;

      contract
    },
    async move {
      let interactions: Result<(Vec<GQLEdgeInterface>, usize, bool), AnyError> =
        arweave
          .get_interactions(contract_id_copy2, height, cache)
          .await;
      let (
        result_interactions,
        new_interaction_index,
        are_there_new_interactions,
      ) = interactions?;

      let mut interactions = result_interactions;

      sort_interactions(&mut interactions);

      Ok((
        interactions,
        new_interaction_index,
        are_there_new_interactions,
      )) as Result<(Vec<GQLEdgeInterface>, usize, bool), AnyError>
    }
  );

  let loaded_contract = loaded_contract?;
  let (result_interactions, new_interaction_index, are_there_new_interactions) =
    interactions?;

  let mut interactions = result_interactions;

  let mut validity: IndexMap<String, Value> = IndexMap::new();

  let mut needs_processing = true;
  let mut cache_state: Option<Value> = None;

  if cache {
    let get_cached_state = get_cache()
      .lock()
      .unwrap()
      .find_state(contract_id_copy.to_owned());

    if let Some(cached_state) = get_cached_state {
      cache_state = Some(cached_state.state);
      validity = cached_state.validity;
      needs_processing = are_there_new_interactions;
    }
  }

  let is_cache_state_present = cache_state.is_some();

  if cache && is_cache_state_present && are_there_new_interactions {
    interactions = (&interactions[new_interaction_index..]).to_vec();
  }

  let result = raw_execute_contract(
    contract_id_copy.to_owned(),
    loaded_contract,
    interactions,
    validity,
    cache_state,
    needs_processing,
    show_errors,
    |validity_table, cache_state| {
      ExecuteResult::V8(cache_state.unwrap(), validity_table)
    },
    arweave,
  )
  .await;

  LRU_CACHE.lock().unwrap().put(contract_id, result.clone());

  Ok(result)
}

pub fn get_input_from_interaction(interaction_tx: &GQLNodeInterface) -> &str {
  let tag = &interaction_tx
    .tags
    .iter()
    .find(|data| &data.name == "Input");

  match tag {
    Some(data) => &data.value,
    None => "",
  }
}

pub fn has_multiple_interactions(interaction_tx: &GQLNodeInterface) -> bool {
  let tags = (&interaction_tx.tags).to_owned();
  let count = tags
    .iter()
    .filter(|data| data.name == *"Contract")
    .cloned()
    .count();

  count > 1
}

// String locale compare
fn strcoll(s1: &str, s2: &str) -> Ordering {
  let c1 = CString::new(s1).unwrap_or_default();
  let c2 = CString::new(s2).unwrap_or_default();
  let cmp = unsafe { libc::strcoll(c1.as_ptr(), c2.as_ptr()) };
  cmp.cmp(&0)
}

pub fn sort_interactions(interactions: &mut Vec<GQLEdgeInterface>) {
  interactions.sort_by(|a, b| {
    let a_sort_key =
      get_sort_key(&a.node.block.height, &a.node.block.id, &a.node.id);
    let b_sort_key =
      get_sort_key(&b.node.block.height, &b.node.block.id, &b.node.id);

    strcoll(&a_sort_key, &b_sort_key)
  });
}

fn nop_cost_fn(_: &Instruction) -> U256 {
  U256::zero()
}

#[cfg(test)]
mod test {
  use crate::test_util::generate_fake_interaction;
  use crate::ExecuteResult;
  use crate::{execute_contract, sort_interactions};
  use deno_core::serde_json;
  use deno_core::serde_json::value::Value::Null;
  use deno_core::serde_json::Value;
  use indexmap::map::IndexMap;
  use serde::Deserialize;
  use serde::Serialize;
  use std::collections::HashMap;
  use three_em_arweave::arweave::Arweave;
  use three_em_arweave::cache::ArweaveCache;
  use three_em_arweave::cache::CacheExt;
  use three_em_arweave::gql_result::GQLEdgeInterface;

  #[derive(Deserialize, Serialize)]
  struct People {
    username: String,
  }

  #[tokio::test]
  async fn test_sorting() {
    // expected:  j7Q8fkIG1mWnZYt8A0eYP46pGXV8sQXBBO51vqOjeGI, mFSUswFVKO8vPU4igACglukRxRuEGH4_ZJ89VdJHnNo, YFlMzDiiGLJvRnS2VSDzqRA5Zv551o-oW29R-FCIj8U
    let mut interactions: Vec<GQLEdgeInterface> = vec![
      generate_fake_interaction(
        Null,
        "YFlMzDiiGLJvRnS2VSDzqRA5Zv551o-oW29R-FCIj8U",
        Some(String::from(
          "J_SFAxga87oQIFctKTT9NkSypZUWRblFIJa03p7TulrkytQaHaTD_ue2MwQQKLj1",
        )),
        Some(743424 as usize),
        None,
        None,
        None,
        None,
        None,
        None,
      ),
      generate_fake_interaction(
        Null,
        "j7Q8fkIG1mWnZYt8A0eYP46pGXV8sQXBBO51vqOjeGI",
        Some(String::from(
          "Q9VhW9qp_zKspSG7VswGE6NFsSgxzmP4evuhGIJUqUrq4vBLYCXrPrYcE5DwSODP",
        )),
        Some(743316 as usize),
        None,
        None,
        None,
        None,
        None,
        None,
      ),
      generate_fake_interaction(
        Null,
        "mFSUswFVKO8vPU4igACglukRxRuEGH4_ZJ89VdJHnNo",
        Some(String::from(
          "Q9VhW9qp_zKspSG7VswGE6NFsSgxzmP4evuhGIJUqUrq4vBLYCXrPrYcE5DwSODP",
        )),
        Some(743316 as usize),
        None,
        None,
        None,
        None,
        None,
        None,
      ),
    ];

    sort_interactions(&mut interactions);

    assert_eq!(
      interactions
        .iter()
        .map(|item| String::from(&item.node.id))
        .collect::<Vec<String>>(),
      vec![
        "j7Q8fkIG1mWnZYt8A0eYP46pGXV8sQXBBO51vqOjeGI",
        "mFSUswFVKO8vPU4igACglukRxRuEGH4_ZJ89VdJHnNo",
        "YFlMzDiiGLJvRnS2VSDzqRA5Zv551o-oW29R-FCIj8U"
      ]
    );
  }

  #[tokio::test]
  async fn test_sorting_2() {
    // expected:  hwwRzR-sB89uQ_hU9UDViQYBmUg-tyf_1C-YmesZbck, ObACsVmx58xdmsH0k0MCdKdqPXyaT5QJl-lZLkjGDjE
    let mut interactions: Vec<GQLEdgeInterface> = vec![
      generate_fake_interaction(
        Null,
        "ObACsVmx58xdmsH0k0MCdKdqPXyaT5QJl-lZLkjGDjE",
        Some(String::from(
          "luiqFPm09idjhj9YiNOxN8MvTGcWLa2oCYPa9WdZsFuJi06oHgSqJ3wv3aXR8Nlq",
        )),
        Some(741972 as usize),
        None,
        None,
        None,
        None,
        None,
        None,
      ),
      generate_fake_interaction(
        Null,
        "hwwRzR-sB89uQ_hU9UDViQYBmUg-tyf_1C-YmesZbck",
        Some(String::from(
          "luiqFPm09idjhj9YiNOxN8MvTGcWLa2oCYPa9WdZsFuJi06oHgSqJ3wv3aXR8Nlq",
        )),
        Some(741972 as usize),
        None,
        None,
        None,
        None,
        None,
        None,
      ),
    ];

    sort_interactions(&mut interactions);

    assert_eq!(
      interactions
        .iter()
        .map(|item| String::from(&item.node.id))
        .collect::<Vec<String>>(),
      vec![
        "hwwRzR-sB89uQ_hU9UDViQYBmUg-tyf_1C-YmesZbck",
        "ObACsVmx58xdmsH0k0MCdKdqPXyaT5QJl-lZLkjGDjE",
      ]
    );
  }

  #[tokio::test]
  async fn test_sorting_3() {
    // expected:  0J9RP8MwtB6y-Z3oRNgdURHXC2p5u5pm3AGJH4JBPLM, RqBAEgQtmpIwV9SYqxh0eZpil85Hh1ILhVBg8B9FmxA
    let mut interactions: Vec<GQLEdgeInterface> = vec![
      generate_fake_interaction(
        Null,
        "RqBAEgQtmpIwV9SYqxh0eZpil85Hh1ILhVBg8B9FmxA",
        Some(String::from(
          "20vRhS4EjGRgI66joMVWmYZS9hLflTpFV5iQvFKGFbMYJBPxM8kptBNI9Pi4wcEU",
        )),
        Some(832275 as usize),
        None,
        None,
        None,
        None,
        None,
        None,
      ),
      generate_fake_interaction(
        Null,
        "0J9RP8MwtB6y-Z3oRNgdURHXC2p5u5pm3AGJH4JBPLM",
        Some(String::from(
          "20vRhS4EjGRgI66joMVWmYZS9hLflTpFV5iQvFKGFbMYJBPxM8kptBNI9Pi4wcEU",
        )),
        Some(832275 as usize),
        None,
        None,
        None,
        None,
        None,
        None,
      ),
    ];

    sort_interactions(&mut interactions);

    assert_eq!(
      interactions
        .iter()
        .map(|item| String::from(&item.node.id))
        .collect::<Vec<String>>(),
      vec![
        "0J9RP8MwtB6y-Z3oRNgdURHXC2p5u5pm3AGJH4JBPLM",
        "RqBAEgQtmpIwV9SYqxh0eZpil85Hh1ILhVBg8B9FmxA",
      ]
    );
  }

  #[tokio::test]
  async fn test_sorting_all() {
    let smartweave = r#"{"eonAuNWeUzSCYbP9hFXJd5GwPhx2QQIWlf7LUOXk3zY":true,"70qLw4Cu4F3RFXBa671qWrrJF3vvdwAunw4cugvNStc":true,"cCCaq6LyxwOF2mkf68TbZlIPO1md9bT6VbP1zHCmsd8":true,"C3ov4Glyo1taISVqJNZvzShL6f8kDczESP8goH4L92Q":true,"tkf88hPmG6iHKAMnQD9a6MRCcd0RW7RxKTk-LF3uVno":true,"0ZIF6dZc5nEsglSnuHIhBR9I2mNRawJYAjPPZe3xfWA":true,"oQ4yerPlykJXwYVB7KDOsttrkM3a6uPYTHwEFQCNpSI":true,"4JcLcpN0Ow-gecMifw1kxDCP5ROIxrhgZBXIWVti54c":true,"io5W0l4J2vF4P7hYo1jiqs2dVUvhNYvVEj-WB45WmPs":true,"hwwRzR-sB89uQ_hU9UDViQYBmUg-tyf_1C-YmesZbck":true,"ObACsVmx58xdmsH0k0MCdKdqPXyaT5QJl-lZLkjGDjE":true,"m_U4IhmgEC7r2kUUHlpzWO1tK6ZtkR4x36C6Ok_89rk":true,"Rvs5BFZ9xk2VfUsGh0zrqQpXrqkOC5kdDMtpe-FT6Lo":true,"gKnTYDaDqGK2qboeUkuCCo82jstQhmp99vpvmCFRAXA":true,"dfXwt1D1u_2dXFDbNE_uHNgsj3LD7_ljzzPMG9dThjs":true,"kA0spbetH7fRcgLXUko4UEEM1P-NeqgsX6OSJrUO_yk":true,"vY1N9AlpejaIrPMLLXd6BBceWsATUm7cdvSazX5vGXE":true,"T97sDZ_YNalB-mTyD9FS6jlNmjY_ryG6aybnX-NCSXY":true,"Iq9SWWchPIF_mdsMoylWGkEMRsCEdcDDOFOquyDydJo":true,"QzvsVIslHtO2lBxctHRyEsaCEWL43GMA70Tg5hxLbnk":true,"BcKnkf0QXep4x_WmMqUlsF6zObOG3D8egppEaFqzYTw":true,"k88paY01zELCpbNwCYl5Rr0w8Su90ActiNnMHcv6cw4":false,"KHUu2yGz1F2XbRSWh4IH5OO6mhMI3GPoCPsKI-KSdcQ":true,"XYagdK4Ftc6RboxBFLrtagXQwW02UAMMHSfwKgVUMbo":true,"f4-ech_RYnvA5_L8N9Mm0pnyPJcS2RqK-wuCrqrgVJU":true,"dv9BFzCIesK0u1fVvdgZqx36dPGRg0LHOlWNixmq6tc":true,"z32TrD-nMubPIaVJV6JMK1WonYRAxKWbGvNLN5FbHdc":true,"SuvriF1UxfTTHjWbEno4kua-nCsAXuSx5FnVlggJA60":true,"67gW9CoG6d6SbmE-vqm9zwsSvOzHucckomY6ZBQacpk":false,"JH5Ycr4AWswaICKcswQx9bTFeJgsQv_TNg7j6yEDG_4":true,"ep_Ftey02dC0WDeh0ufgqJZdLlmz-z4O9lKOe42BK0Y":true,"SYC4H70Rjk22NHkk1ehDT9Yp9JmKpLsLRiW12fxRfq0":true,"blnBanag6E_S22wJ4Himxzz4kw9TnywXpQXQWCVF4zM":true,"v3Wq7Pw-jihZk-UV0t62jEiqQCOzcnKt-uK5CKTCSJU":true,"3XQQMBThPlHF4sK0jyKZu1OzGEO-JYVEQkDJ-90tQW8":true,"tU8EXg1YUjzrTPsuKShPEzo-rwW60xusVjX7t5abk_0":true,"62f1bUbMLTdV6ZhwoD5m3zBofsHYTXb0elYNuWr7z8c":true,"e8Btw23fLxqXgQSUpG6GlzPvvFT6huqqCljOygvf9NU":true,"8_GussqJwSH__Li2h3AtnXRSLBuWHgZbRrN9-J29F54":true,"OVSm5pl0bwfpsbXlbVNK1WDbAteePSKNKGSvEwXdYzY":true,"H0pluGLL8XViIAz_xMXcXyypXGorAHmjheOmURXPwWc":true,"GMwefoY2b1nAxzUmwawVCv-HJqHA9SBYSumBGXymjnY":true,"1pGOkwgtmlZJoZYF8Jt6A6lDNUsAtsmhFc5FEDt1D_s":true,"j7Q8fkIG1mWnZYt8A0eYP46pGXV8sQXBBO51vqOjeGI":true,"mFSUswFVKO8vPU4igACglukRxRuEGH4_ZJ89VdJHnNo":true,"YFlMzDiiGLJvRnS2VSDzqRA5Zv551o-oW29R-FCIj8U":true,"Favoos9niGz9AFplOvsmZ3CQHdaU04ATah1jW4NJVtM":true,"AKc9azn9mm0UfC9kYssprLW5XPC4IXUw_5OgrRYbQmQ":true,"5iPKKrnxMBwZAiJDienmViDJKC7NOJz1mGe9SiplgxQ":true,"VWGE6bHzMDw3hmJSlJgBcjmstj1Lb1DaW97pvi7VD4U":true,"UrPXfEl5rOjyEdMXemm-x6xpt-B8fdIuYPYShJ_SuLE":true,"9rVrjCQ1-fIVPnFMeQMtsCO9lGKBIt_0Jnl6IPNYmTs":true,"zTZUxPPEub9kVz2NYNQJJOBK8RHml9KbR-SL0-XJulQ":true,"8rYzGPOukYe-nKbcDL23zFUyvobxz9Xa1M_SesxpRAE":true,"MfNX5fcKo2QG0Xwff_wH-_IJHH3fbLpVIDseQmAXWc4":true,"nsY4kALNAxQGwHzLd6Mx3yC833qO74or8sHFi1iFThw":false,"UGU8BFWBv6Zy4rXj0zFyJR7jF9OJfsjatMmYtAlE9k4":true,"P_XDmjFVhuq7WX_b3eD05KIAVKoEkJv7vGdkrWzVCaI":true,"a-rHr8UZE5GsnnlaDuoaosElVmDMgNOHrBU9w_x6wMw":true,"0AQm_Rx8RDbmvcjCSOMTUShon8dRPppEi63PTOavaK8":true,"ER2OR-KRdAQwqaJoOX2pQqdL3aHuFLCKxJDwmlIlvhM":true,"Thc3jfs4JkygJOO_iWOMjuI0G6_9Al76DhyN3Ad1se0":true,"LUCrGrsvoKzp4d7-CQTq9C-ykwdmnRx5TRKmnCBL0Os":false,"aoe_5h4OwcasI1wvxFsBqOUpHusb97EXOnZhSZJJpv0":true,"pEJcT-HJxf2FwIike540U5yX20A7DF_iFERb5SyKr_E":true,"4wGZZL4KTsXdvUV9BdNP5bLO7qZAtjBWydN0XTgMGNU":true,"jVURc9U_KajwI5Dy-AsPLjXMXjnaC1LRjOlPewXiK1Q":true,"ySEK2a7RCFfFM-OW8xLmWc6vypNl7NfvQCZ3hDXRZMY":true,"QY9XWoi6sxWswKgKRsSnta2bFuKrAS90Dd3mXG9uVwk":true,"QGX_drj28-Lt_0_oOCNqiju1EZOzg9-qP702rlGfM90":false,"OvdX3r4VUPFhZyBN1-tn6OHvZW0M2nXO9MLOIVqMZVc":true,"8O2NqFWq9W1lFy_RL84R8JotbiQIjm2LtH4zid7YGYY":true,"yK-0xAXLVgpJYdw7xTRmjM8Any8W4aLjCuREkmM5geM":true,"YfKvu_b3Cjqy1DYK5nnrJpe_cmLkcvH2BfPcthT91bU":true,"QuKWzQf9JKH5qZuVzmA3cwi-Bchzx09X-V06gJmUloA":true,"XT0eUhHIRUF1A9oE6qb7gppA0G_5GzhwzEGJ2RUqDzI":true,"lAHqs6eZqURmPC1cvmO9KQ4vTf-XxxZFmSD66OJys-E":true,"-9t4FA_TlPQVqVhiAZSJPm3Bwtx8AAIBcSPKVRzolZM":false,"OSUw_FelYPXHEVnbf_0nqgNhQ2E6CCA2WZTmEgPKruQ":false,"oynwh2XjkcCaJraTbrPLnSIjL8WDHV0iAn21k6JceG8":true,"gaOsxfUbdjWheMnal3eyCQ1VA90i7WNcoIIvsRhz9m4":true,"Pi5feaHFaBdgOnn2JUIrIBfjvevMspo56HUEYccsuwI":true,"K_xc-N58nIvG7uz31GGlNoGUUsA-Tqmox5d_-ILcb3U":true,"NCdARBeHlomAsYGqyoTK-lKCsZtl7LqctCCqmbq_GGk":true,"6uMnm0wfJSM4i2irczCO_CgkciTzu1b9LUy1olSt6MA":true,"SSsB46s4N9rAuUjqSdY_62uHyJHPK1SZ6URu_GahLow":true,"LCih2b8ugjjSNGBj15Ap8164wOwl6D8M7J2RhhY_6sA":true,"z0zWufpEfmKlM7q6i7f7Rwn2Qj1foWBHXUBbWAZB3Xc":true,"ZiLWX3lBcTyuDGWXffEru8e43zCH7qSo0ad0CJyBtXA":true,"FZg4DCIK_q-ldMhvYQJTJSlBTIHnsvwNaLuipeRaLrE":true,"U4VzyeUemwvdjUD3OT8vkMIeGje0OmUFczzwkwij0Sg":true,"OZhahRggWPN9nw54UbdLag_BMqoTGq6quoV2tp515_o":true,"gIFnKtUBrZdafp4kVIGshUlf6FWEvt_dgf2hO2wDWA0":true,"ybf9jlf2vASArNmE0QW5N8wA4E4YX3LAzWQupxta-fI":true,"d0VTcdi84sPzu6wzszS7qYEwugyKYSSZNkPwY5m0Qis":true,"umdBQ_ifPMMnasvK6i_vfa8ZIxomRj_61rDJ4iltl3g":true,"OVfTk6Sf7Hm_IKur3Wl6cX9_eFKiXOUsCSZdPZAL-5o":true,"_wM-bvmOK5hoDsr5rAM3mwe4dnuFu9Cr0TMA_EH3x6A":true,"ccw0-cwrMYF_zcL5WdPWIV9IQS1SoDcf2_qqeT545uU":true,"vALBERL0ZNg821mlD0b5LKqRB7Tq7n_A2dE9SKq15k4":true,"vkxYCU9XjJNapy6m6MoH8q25LRTiu1K-ccYN1Mo80xg":true,"KkHFfuAgsHepCnPzH_xYu7kqLwB0p09jkcoUVvqUREs":false,"z3Tm8iPNAyMRXHxESqL9JjBRG0XMuhWVFVkC4CbQKPk":true,"A190kIDdjx43Rpf33LqJRQzy13pIUHpCEo-_uoQH3E4":true,"JY62lJEVkAki6K191uu4II27O_9lTrEGAGix5RCsspA":true,"8C5w4FP-qghZdVdDCABN7y1BoOzX1-v9SDHoYKb3t1s":true,"7kZvQGX5n89RHh_EvvAqdBl7mgcKXziDORn_dOHk9Lw":false,"DWFGFIskc1jLvDZ5QZFUJCWWXAIBfh5HAaTeOXpTRGw":true,"pCq9HenlyMpuPfhW3uMyT73Bq7Lde7TsgqfWk4_LrGs":false,"SNFkGXxYITRvYSvOHacjMFhTQpHf_E87N100K3DsUuU":true,"q0eU9SL6H0kiBd1cqQJ4NYUYOtZXELPe36A5lySiY28":true,"e_T5vbMPH9CalftP1NnzPLGFDDZQ4jVh6Elpw3gTL60":false,"vOv5PavrJs0CwA3TlXid6H8iYhj4-QGuqhAxeuN82Qo":false,"aDVWxgHabH4f1APmgYwoF00LKgX3NjVd4B9YTcxvVek":true,"w3ykF3QDhedkELNQx1mAmXZ30PruLqcl0XvHULYDqCM":true,"UClnTPiaFWMi8MUoZwEPulJoZ6d-P4p62ELvj3cvrmo":true,"1Ue7e8oBh5CXA7gjDFpKcGfm7c5azrDWRtIXR91YDIk":true,"DALK6U5uvaGpx9U73XGHuFmVVKT9Yhp7GObnm4hi4ec":true,"M6hvSSAarY1BVVQGBFdYUoI9KpxZ1aVrkk1hfxBa40Q":true,"IlfhEM-DTFIXtWtM_A_1WBqPyP8L6hMzYlgIQ9RJkfs":true,"HFCgMgn3YnNI8fVV6Wujytrd8wgrlvEDFEJCwy4ryD0":true,"mi-RzgHQylIjJRfo62CuQYSJWkhGBpLivJDS7NtBuRs":true,"8vFAzh_vRjSvdh4OpQrZmFWsGXHdYA2wBUaGn5IstJY":true,"tqBFOt-ZKbVSoTtif7R7D6ulAEASOADEnv0IQ-HVkwI":true,"4a8R494YoqT4MCyJnLkNvaWmga28u_i8S7aWPVYhJW0":true,"tb2teuohJk7oJZEg3imRoJzg4r1GHMPuW0CoZWqDaXI":true,"BUx4LRe7zc0wuiX1D1_ai3IFLCztx2qckMwQTeh_x-o":true,"beEqqogn3U2SzHKL2uI_96qBqf3BJwfBP_hDc9jksgc":true,"3N9NkR57-DZ0Zw4E0hQPOW8SLlCss159iW20DU4ZbM8":true,"YH32IUTDOckfnMf6h3p2n3Q_MvrlHL5qeAW_jfpdg_0":false,"ZJyU7-_uUAbsmyeZWcRipzP7WU1bvbblmjfpb93SsMI":true,"pPV5wtGEKJW_wU9dUMtcm30SuXddSI3ixZWCzCjpQnc":false,"Z3q9eafbWGTofZzHns38YiPcZsE8dG8mUtDRHqq0PM8":true,"eyWZmzm8l6JjmRH51_f_PyybVO5rKDDpdvmCwNHJRBk":true,"Q8ZPJ9ddbLMI7RlsrycgZXMa2ef-wQBCKVjTs7pqb0I":true,"pe59tmov8kabG75Iotsm6Qsj1F3KiK5wnY8ovXZC1pA":true,"bi7YhdPVuDMWik1m5kDoauuJHem17IwwKHUe9wkIBZ0":true,"PqFDq8iYBBa8vRTmmz_I-z5V8Vn4istxzNbj-Ado2ng":true,"F5ZLBgHtaateo8M1YfW_bN8lFuPkb1nTtycfCwvMivU":false,"8-KvM8GzD5XMU60dIBP5_AmSItlie781x9kBHke6N2Q":true,"BVxZhRNcY5NXSVIEOeoMGV47HcY-e18eWEvw6Xmsdmg":true,"3tKG3y4gPOPZCmYccWrGPT5b-T4GLNx74T7vPlUuAhg":true,"s2wlboGtyce5QrrYukvWO0Yg1hW_SfX_iAb1n2PFPFg":true,"xjDJJPUl-pVu4v-Dc3ADnpT3iAx5ngcETxGLwoL1Nr0":true,"H3mMpkQwSJUrw2KrRCFyQO6ICfwrYjHsPK7wylv5xtg":true,"QeEwq-0iha6gfjxI4Ruk-RPUZCWH52D41L9q13gzcm0":true,"aS4v40dlfbmxPtT-OEyp7pxmQ3JoSQqtUoqJ002OIiw":true,"lT9Dt4kM7sCkiS0s-jlxJ_psfUU4XzeogK-YlA6k9rI":true,"pcF_7NxkDJdja0Y7UQMz1r8gma-mBdzX09TSQiDGlzk":true,"-XFESlHZY8kyNRcGMu7F0M8gyYu9TohMZ9KChjoFVH0":true,"kRuxQBme6eOpHg68QKx0YWZB_IIsLLhDAMh3g3KyXA8":true,"vuX644gaFva73WScNUhH-fQlRuh77WdKEwPcBgAbprM":true,"54IfjBhpImdX4GDQjiNPwJ4i-NvCVIKxYMhqnAgdb9Q":true,"RIPimQy5cFf9KYq-2YkvN_juc7Gdz5i6oO9q7GONq8E":true,"F_d2x1cuu2leKRZh5-MfCVYkZpaNBcLiKuEyYOKyiEA":true,"eK4-_TsRJIm4gN0BN7Gbqx082-f3f8DQicF4JhTizJo":true,"LVRP-3bFPgXgSWY-05KoYS8LQYOTpQZFJ29ZU6rwXQU":true,"oBY4PS5Iw9jp4nQ1HFIJ2vUg1T73hVfvLrW1bLgNTbY":true,"oJAe9Y393_Xd5f6KXdupZ6J9J4liqxJt7EpezWwKnHg":true,"Xwl0NW4t9-GKs_AeLRA0P1O9xln548Yj360BM5QjI9c":true,"4XZ4Na_h9MRlKjjEZNgCNE1XYGcyXRJEIEGjoK7PVxY":true,"NBmM51YDjffY3V5SyrCgLtQ9P3A-HGY3vGTciGcKYjE":true,"_kGrcztT13VxtCO-Ik_GJdh6IhZ6ckf_WfDcDxZMGfI":true,"RYLNyM4Yh4u_AnOdaxyL6q2WO9YAzWliqxsjKHOVIto":false,"OE-9TVZWO-5WZR_2OttxSsPxyPlCmg_-_1WrHUOsCyw":true,"8ntCYouZnUU3k6pq3GpPfsxIx0okagq6RGYP6nr6WtU":true,"WeHJFu5DcJJNc4VsU6j798JqR8ULB-AYTuh0FF9CsEk":false,"q6Maccp-uuaVAhnJB07urNUq4heHSu96jQfdI3evRyE":true,"yhsMoopzXXCGpQJmpK4XS1FJT3gz5BWx8H8eFFQJ24g":true,"WNFIeGcnkDpvjtCYnwIjLAG9KQBvrVBJhvcZdmaaEmM":true,"QwUjlVqK92M2yERQXgrvitsSSD5hZGWDUKcjgUrausA":true,"lnNikyurwYKLCrbv295MYkvzVmyyXz1I9Cw12aEsNc4":true,"E1k83QQ1WzdQtvZ_IZwwWEwdAyOcCbZYVGxx5yqrsC0":true,"2waaqWmHWpVGuCUSXOLqC9aXL00rQOzLA7tMeUeALAE":true,"SgVEQiDXfjebWPN-DKWpdAARp9YiMLWBLGkCWws9E_M":true,"1C6KnxwThH_KRQ7fMlpjde9i2Jj_mLOao5J2ex0lF1g":true,"qg0NRCyyZlrz8GUe-slQqHh5DEvmJr3SHJIi8CDPUpo":false,"XBg55n1u_Xtc-u202s6NhisstX5-fpcUWlBlhrSiz4Y":true,"vaOcUjb4pOUWVTWkjTNse_jIovfbAXRHfx5EA9y0eik":true,"ilkzINi2GMerpcKTSEWBvLDAk_Dou_PDND-kupULRfY":false,"fkyA0Swc90Lfr6MHNpa3XeSo2QCuf5fVMl7QxEu69N0":true,"tZ8_dmoSbCwE4h1uoEKHt4169S-vB4ygVREosnwtdaE":true,"9ZJVtMHDTUPzvmWgHHNvW1Q4wnXXq_09h_5wRfvBsrY":true,"0Uem5_7loM-SeSA7K1E2ioDeCTZ5wYdqtCopTJ2J6BI":true,"kj9-QXe8HBOYj6om5nJWBOm1s2P_og-y_Is-SLNF7bo":true,"576hnml2aA8XiG_ereIyhbyPVWqkdeD5TXT6PmTOAeE":true,"iExv6YGmHvCo_v3BwXBpPlLKVRCuXhvtTCznU8NdaRU":false,"gklMp4QaKBa_aORgsAaTGWUV-y1MQCQVUqWMmSij-KQ":true,"xg2V8Ydk4w0jZpm7W_pReHnHF6Whs4nEppz1-G-hYtc":false,"YiDOXI3FBN3B3Oadjk9zjA0yhv0ZdYLw3MlzLmGaOnI":true,"s21RSRJHOEGRO8wuq7JQcbCTYIMVr1tlZ2LRW9CFH2Y":true,"Ma2OpyIjzB62tUkvHihekkzxhifjk97XArFiOI0xh0w":true,"1St3vDd4X8nqMv4wXre4V7oJSY8cydTiKBs7C-ChXN0":true,"XzsXv5zy9K0BT729S94f-t6YzSSnMQnl8uwFHhd3C_k":true,"HaKOaN_v2mpo3DvMB0YppQMJC7Z5XiNUdtwRQ19IFlU":true,"Lo4B6WzS5fHxVVQGsmtUwHsEyoRq43MEAz37StJzPu8":true,"AP-QO7FMnu0MjLiwSa4qw37aecG6YvWgBDhwXrkpqJg":true,"MyGwAFpMT0a8AoP5Cnn8ptw4mBV-s-VISI8xwN8scOw":true,"HC4yvu5orIZxfGvuLhZ-z_6pn3krb1SV1yCuQ4YtNtA":false,"lmyZhTHvSwJtdXAbQKzVvpdfV9iL1izRMPPmgNFhAes":true,"GwjmUjPyeKzAwQEGQYCk-mRkIF50D9J2oAnOuRmVTJw":true,"KE_uYXNkx6vxDyCMrD8UHllulxCioomFlC2B8ZWaR3w":true,"n7UCugShvMKYfpGiRq2VrXLnfys9KRNg1qpIqXdLd-0":true,"9dKhrxfcE3YaUH7zvL_F32kedWDzTCc4kTsN7TJ4fLI":true,"vG4Se-OxprqERRK2JQR2f-T2kDKhvvRzE00Lx20FgwM":true,"lyJaxUYTPZwpLlhU21OGkWOEOntvmRF5z-fnmgmUQb8":true,"oP_GNTeofCyseSAEQZvacyGVXKvMljpWYmmA9K6N0O4":true,"Q6rjM9t6n7GchVhQdp8COVlf6o0C0yZAkJoKH4f8rpE":true,"9oEZzZcIOFTr3l10eIqNhDIlDMotJQTPzUS-otbWMBg":true,"PYlKou0XgatKh4nLmgEZboPVX86hB4t_OMq7h00c5FQ":true,"nX0Eo_NnVdvmvzli4oX-_UznnxCepqJzTqTz71SiatI":true,"dnDrhA71nwxx_JJk8XhkUE2fFVfrEpdKFDmFggbcflI":true,"4ULw02rCVm1sVIXYXf3pQc4j06-HKPyDr67lIHz1jJM":true,"UaiKBdPlDyMTGzb_PLv3WvAgGBhMbZhdLuIwT4gP4Jo":true,"BTqI6psmWfQe19udE-Kt-BtBCn8wAGK9_vthrjBJQzs":true,"Dblhs7GFW_YRIw55E1I74cY_5t4ZZaSW15n4JbGunI8":true,"BU9YBHHEb2QDjQ4gRif1dfsaDCGodnpmGvrA72OLtM4":true,"cPL4NKE3I-jOJ3DqBv4jP-KVjby0WBVlG2aIDVBdYLc":true,"upZVad8wNtVkZJWFlM-7RdJU-2SZMTfA4IpsD1LiNC0":true,"jmxd9JULhETMYF0lLRjh6eEGpF-Wu4OVqQbKC-cwCT8":true,"IMnmPu4x7eX8d22WV9Wzll6GgEYk9t7cvImpZHDMMZ4":true,"a8B8OgLriBYcoAq7dQ8sCzrddfSXGv0O6ijM40hJ_h8":true,"sidLCTjzNf8CYLPEbqJWVQnci5Wmi2WSvKO-mQhVZmc":false,"gn-1QrZY6KYILMSwSfX6PvY-vAlNZawlugN_-kVeYnI":true,"18EIBbl1joPd3w3iGe-n0zdAwMENDAdu3o0LnrcuAEI":true,"e5oqtDTGxmfvJfjC8kZVai_LBS6EKzcxg4ymNG1y-fA":true,"jmiUM66zvlvhP1gsY2_WXLs0wmmFZ6pPrah6Ro0pw00":true,"0Pn92aJmxGxmCcarnLt9BNWNTIPFSYa-YGNdBhCbJtc":true,"-MJAYY1YjFk4s3C8vN7uOivI0OArzeQMZOL6nKD9pzc":true,"0RP9iHWSr2pNvz6VE7E7PKXhYfD7yiQ-6UhfFoccDU8":true,"o6WeRU-kSGYGhvlwWE13llBeJkIauaGQT_jSJZ8G5mQ":true,"yRT5fTN5uzYFnDvW3gnLpyOTp14siX_Ba7-WSf92Csw":true,"EjepBue4le66Jl5VoXdfuOZ-3wJNdV8o-e77oGJsNrA":true,"Rmml8VTcWb9UIpBYeN6ZNc4oomPZcYkz2U8MVxw39JU":true,"eJCYZIUru5wGPc607Tutk8BoIkGXpbI-ILhbDYytOt8":true,"IwYpG61nyj6Ppr2mq1KwlbQmBwtkAkUTQw6jOrzU3aI":true,"uVEH6OYNRq9yCgZo5iRkLmt3oH9-LoMmm6jd6izldVY":true,"f1U88VG_xjlzXe54PZPa_I4OSqgKXY6GCEp73cZkLc0":true,"90l3H5Pm8eSoTTJPcENfOsH0Rg4CLdRn6DUF0pN2ADM":false,"Iai-BtZ6l8ZZgbQZx-sQszVSItzMGdPFShKYpo1EH0I":true,"yreWEsZoftZPIFVqIWZvUYT72rSzAkucRIiHDBo1dQY":true,"0at9QFtkyTTv_hztHbhMtxV5sPYe8NicRWDPnncRE5s":true,"qxsXmvC9GrVSiaUr7QUYnn8dcNEm0c8Tm4z_3o2vYWY":true,"mqs80Q3RRdEoqfZZ0THpzk-4VVGG8E8D_ntVOCekztY":true,"2dvvhjPpMcF61wrDR8sVmKbLVMv74AkBDqP890Z5f6k":true,"cJjlJlatY_WBhq6XoMQaM37Svm_fyW_vM2MiUXQ-m6c":true,"AqNnKdDOQYCimAQE0FsmljTROze-Chmo77LkjzuWoFU":true,"h6wpp9_mUqj5TTvLN424bAQJ1kWNjQ7bS_h8nfZIuiI":true,"M7mtygYgCO6hg2Lr_9bQOsongPDGdJTkJj6swroAeOc":false,"uPMFcMbXhOKUihFYk0DzOWh3xtpj3rrc4I_B4MlDiDs":true,"-P7PyvO0JpDeYxYGxFtUHRPAp2_fkh3p1DQOhFZoD9I":true,"i9ou6coBNq-rx1ZfnHBgXh4mtKzEHto8kFv--ZXWhCY":true,"YT6_Ye9AmJf1JiHvxZP11Z-VrqaEuxII67V2i3AWcWA":true,"8s24aFccMH6XnEpG93k2pX6f70JllGjl4XYe_0KV_0M":true,"0wIVM9TjY8BKlgPJ6EWX95YT8FH16ggglErzExNVAks":true,"HOJMKq50AtkzRmG5HM_41ntEm23y-PqrGrRjRM8dHEk":true,"jOLieN4a_CVmCpt02O2EOSGyVpxVOAHDV-JBV5AMi1Y":true,"rlZNNQttwZ9u-SC3uAaORLmQchifSprHnummCh7XO0o":true,"lDFN2w8rJamuKQ2gReYkuvxBmuB7lKZ4ra9jCrFtlh8":true,"NfBHoslT3rzM-DE6ZLOYyUqfSIhVLYHK4cXTd9V9hpc":true,"46ypAvpn_kpdMlvrhFEspYy5QGAWKxgGYfoCKWxUKkI":true,"KsDnOzeBf4BxLKgV7xhXH5Ho5oDnWGRXw93Ew2nTAFE":true,"Z15VOTw7jHUvTU_lJTHjUj33u7m80wrbrIoMNN-DwXY":true,"z2HP0eXQxIZrP8b50kZfYfEv37jqXyLCvblNoYnKe4Q":true,"0VMbljNfBfpfXOgAuR1323jedayJpJoBnojTSei2mgU":true,"yhzODdY1-J8WVtdXI0pqTeE2DsQcJ3aEUL74irNSiS8":true,"ocd11RqLSdA3oxoRG-zAziFDnVbwaATUX9zC-yU27xQ":true,"Vfzw_HUYE6gL6mSeQ6CucMecp5dieCuGYOEHPaTKgFc":true,"VStsJIl8-RUF0Cm9vznCQUg_j6sZKPvsglcWF9zhOhY":true,"olLMcTX4wZ9eE5ifglfdo3J5BWdNqPHBDA0ifoNbn_M":true,"021XrAAj4dW7KALW__rzoXTb75AwoJ3cOO8OIxK0J_s":true,"YubE3-zXdVOj_HP4UYPsBZP3R9LQarzb6r_V--sosqk":true,"EL3GrspMQujtmHN52BNu5SJ_UHdHXtj67SMKPS2SJqg":true,"n-l1J3XrJuf2hq4ScfyhSxuopjfjPwkpCerQnwgCly8":true,"ZKkmZDdbO-Vg8ofgxj94MF3wN-XgFmg9C9DNQG7VcXA":true,"a_Yu7TAcTlxiSKmOxfjMA_J9FRxrtSWpxstjhyn_Sr0":true,"lHqjsQWacbn-W362pMI98QmvgaTmup8NmN2TwV1yCvY":true,"Q_TN5vYdUNQ2Mu5U3TgW1aWm6XdGuCGpt77-wkS5DAg":true,"Q2pqMAXQveiqCA-BbzyKPzrP0vfNiaCIO7iq8z0ARrY":true,"zxyv7lfQRHBRJ3PJcl9GylLp3kNJ0DUuE0gMIwyVGes":true,"2FvNwGKwVQqUp3E1yh6EtWxhf8UZp4e7cKQTYSY80ok":true,"pUY7s13TI_mFQq1UopXwLouDRimcwI6pKm1WFI16j7A":true,"TS1JoUrOJNVt4DMJpS9tSLnwL327fnvOZ39kLHsXzqU":true,"mmFzyB-ERsSQukU3MV9n1FA_Ws_B50FHJh2ujGRNZHM":true,"GA8En-UXRCMLazLdzr-VqYaVr_oDIEPS4nB1jAz7ZoA":true,"qGPOJsUZNsPYpzpwZeerANQOsIQKt0fMUinaxOkfvKM":true,"RHK-GjUKZ-pGr0MvXXz4I-JhYb0UxF-uSkufzjbpBMo":true,"1Ye0U89M_JIn7wB0rQIB0Vs5YdfObFTUBQXH23127ak":true,"Mksf2vwc0INu97nZB41_Ws6cfU30F0XCT0tbwomsqgw":false,"OGCZA92QTfv-hu5Gd5zsnqPAV3Tui81pSZ5BrqZrCbY":false,"FsX2vlUOFFOXQqmBsEugaX1TdBWP7pqcaqqMvuiWEBE":true,"Cpum6HQNRJE9KCATisFDWcCmmf6VvmuIZqn2lFuQ694":true,"KEGS_F6wcxh2BQOudyiPdn3x9TmnNEwliRzR9vP1-dA":true,"9CthZ5D6jQopcqPw2Vgw5rI9-Og-GlhdBC64QNvmka0":false,"D95JIz_ZEQMZ_LggCbPc6zKNN1JpS1yyaFplL6Yf8NE":true,"HOfTS6t8IiOfxbdlqsBEd-qrLOC5aGjWm2lG61HPTds":true,"WUpBd8HCadXH6oxiKWg8fosv5ketsFLLE55jObYqox0":true,"a02YF4wc-1XjWneelDt9K9E0O0wexffRykcfkwD4vlE":true,"_hJKN5quPXTscWlMMawBYcImis8DGHWkGXg-XYWkYNk":true,"KHxIuTdPJgrj6g74gd_WlAqnqJRj8uoI89Fogyy1_8c":true,"uySwTFwkgUMJ5Ij9SHCgrDueyW0fnj99X2FVf3J3bDU":true,"ZYGWXMSGkKbadQ6Z4pVSli0r5OyzRNHJvzHVdriVASU":true,"xwoI2LuzNsj4YEHMk7RdOdSWa_5GDdkZH8XJHbmM97Q":true,"pj5QVWQ3I-LRTTjNbbKtO2XmCUza6hJ-kVtOTMw0vgQ":true,"nu-K8_Hk9uovMnVjsFNAPibn9NVzRUnHL92tiusUJaM":true,"K6Q-oULvEBekf-9_gKJSZ-sXY6jSiYA7g3epaYbf_Gg":true,"C9FwUA_vfAYsL6xM6h8fyfyDIXqDjUjkLDTQkItgPlk":true,"64N8WUxBdpFstdCoD5a7YZwOhXLSCmdPE3yFHrrgnvg":false,"WIkD2yF6z8t8uEe0mkbgSOjqOst16RZ5M-rUTpwBdyA":true,"o9YZg42zMvo9_OnsRS3qEgyIkKgTXS2aDP_ETyRcrNk":true,"ZH4rH4VuXIOgu0PRp6-XDL9ngnQbo2nO1bi77lx-0rk":true,"F5_fZ8Mk0bSrATugs0KwoiyIzvxLf4sHHkOSuPns92Y":true,"9MAuS3AbOzc5NrjcIEPHrNX_izRAOhi36dR4DRk1ZIE":true,"RxqqQp2wlr1fsR8qFGSUODpDFIdrgq2RUqQSSqNjGnY":true,"AEf0whSPtJgj6tJnGQMKX3hKvftDvvDQv5j7WKfoozg":false,"9aUJFEh-UVgR4bcZh5hyYtu1TULasqyNiLTJCYHix4w":false,"9Y0FM_FPSHnIjZ0VOa_4AKjswxQCGB6NnQa1LJ4kQws":true,"srWMdXZN9L_A8To25EsIYBEcPNjX1mkXforNgr8l3V0":true,"yyXFGdJ7QFRmDaOQZquG3VDRTtqTglDGFwEcno_kzS8":true,"gR1vmob9Lk2eKEL9NAYVj0tbMQPrOpxPzQ82sYFDYTk":true,"h4l17BtRr67wKeqoRcEVfPDkpRn-nmeQhh82wkZ4T0s":false,"Hn-NNC9FlV3wekEkbrDzLnD3fyYo2xbdma_lApn-SgE":true,"xClpjF-Y6TB4w30Fu6kz8TikOzkjSVTa4WIfKDLNh00":true,"8ziq4XCksCbVUnDsNY1OQYhBTxICg6BLetkQWejNlqw":true,"WQwplb8IfNgrJtPoSA_tsVtmPUUqduOW65mbwJKDvWo":true,"V_4COoYHOsxGHniuWQ19wiWuwHC0YdGlgpZwil_bTlg":true,"zujx19IrHF0Lra5ZHvrAiEfgQWmk9fEeXzrJYODzHCg":true,"msHLjBSebqDbw1crVVZYmPV16wpsXJCmil9y-OF5JJs":true,"-4stX4AiyIn2ZnO_6J80BAoImyisdVPkZjFrWa5eu_8":true,"PGL62AeXFKXoRT3X5bgl2v7shIv91szp87MeO_oD1sY":true,"nP1MFYWVnyalr_j_Ah_g1dt5l9FefzEPCf1fmOuD92Q":true,"mvQLfTDqY3cv3955tEyyHxyMhbdn0rjkL73Tle_I9p8":true,"s4ildKvqDwimW8RtoqlbcVtxwczPdGclah01qQ8SODw":true,"LrYAehiBY5ICkuZao2vm93KQckrncprbXDA043jQB-U":true,"KYo_AzHKDIp8qXcsN-CXr4lswimzZQ8ogLngTu05Cb0":true,"lM3PRLoZ1xUQWR0YFzyzsvQ7VpdkUFhkvdxK9EUhCbU":true,"F9_2CavzbD9SeHo3uveQLCohRoKdPJ7Z7_-Tk4leYH4":true,"hF5AW57506mTzOU3-StnjHEBgYO5RTpxbGGBHaTVZA4":true,"oI5zqkUFcAByz6wtLEr4ebA-uIVu5F8mMNtFy2Af-gg":true,"Nn1kiYMm_y129gvgFu80uGJBV8JsBz6itecqktNOHeI":true,"a-WrytPuo-QGuIPrd5a2XEZTOvhUIbW2cxnSDifgAvE":true,"bM6xmFYSpfY-WrIVmVVUhKgevzwRLMjcl0QdbU7SxEk":true,"tJPqhILes8WQe6ieowuWFvApLztGiq15L9vieCREyaE":true,"tH_pcE2Qdj4ofzYutDY9mydB1GRfJPeojuAjV_NTu3Q":true,"1645JLv-2n5f8CIJ_ykKPcstaSvkEkiSOEU1bwZkUcY":false,"qOpJ4kUC242O5rRGvGx2fV1QkU4nO60JMM53365Q6rY":true,"e12FeKeF6hKCNzokf4LMOQBLInkGh_XmAgBS24ixsf8":true,"qw9ehI1AlKvkCEPxH7-BVaQXEIUjwYpjPLBeC1UNWcY":false,"YaISibEhYR4AAsB4CUoNWDn_YDZHJqhvc6xYQ_AlRZY":true,"pQn-sGX4fddp43EDG4xXxpwfG5b7aJJRzSIERbIkrhE":true,"bU7nLyjmnmWB66fhFJLhxWMz05zf1NiOaSmrgZ64u_4":true,"B0Vs-oezZ1y54WaFrlesR6PBUGKmGTmA7SCFdvaNanI":true,"bZOj0kUmhH9G3-7sZKe2dFrtNzSQdkJOBiIqOVmPO3E":true,"goBk21xhvMdTF-xffMleuDie3T1K4t3yAjoUl8OEyC0":true,"ld1pVd---VVZFLH-bi3B3DeUjsRyyUM0eYF4zDdxT40":true,"iYB-O32ZZwx3sMcSzIHYkTGWxAEpnRK6u1dedCuYgCU":true,"Xr3x8oOANGTKkyuLvZE9wIH0gSbRk1ParWLVpMlM3qI":true,"HMZKEGt6BZGRlUX7B5XbaJZKiZY6QDtu1Gp7P0j3YS4":true,"H1dA5Teeb4nU7s-scTS8uw5QJTOOjcbPBIuhH3YDWRs":false,"oOElXAbS34WvfZAjNRP-PSRvFRu0KLRXWq_K3KPcX-g":true,"CM0_oEPk5WcmvRHUjBs0_OnbZ9V2J6XVV7WD_4-RwoE":true,"_MxjoDKQWaEf0l7Mq4RamWouTI2dkkiXWpNCifdoHtQ":true,"hd_fior-IjO319RT2WE41UEkMtufsKTp1dNfnMGgFs8":true,"bdGJxVzebPLtcAsYtkGodu5QtEbbmMqcFRNiOQ_pMco":true,"mywc8otEhg89XsDNDwEoKmtS-E3zQjAnOqTejfeM2QY":true,"ZFY_zKVDnx4epmAV2ESJVZkHd9-lGBBI7-EFQkbt_sE":true,"KOc1TsaE0Y1FwgQQ1ptyho6b6UtqngygSYBgmv-xtNk":true,"CoGXAHpgdoWj7zv-rRAtJQtsOZTacxXCJbhgyEE59ag":false,"M8RzfpiW9ix4s7lnCmVkKPLypELhbza36kc2ui-vjFk":true,"uN4MNB_8zAwfo1t_jQa5byxOToadc7XlQzEt5iDKEJU":true,"EWLWFj7Qh43w96kAZ1JfEZeM2fW2_ZCmc3t-3JF2RE0":true,"eBom9LAX3-WKKUf8fEoerU3XpgJCoCyynvZDoPrCzgw":true,"ByHu8vud_WX7GujN6DLlWxr6jM7k0v2R0Fj1JgI9oqw":true,"i5bUUnqs3dBjRY5eNIalZv_9tQjbjH1gbgQjceWs4Tk":true,"5rAusDV--q8vQq0Wdz1OhVcVyXqI2A3SKX3nIx-VAIw":true,"hDL7nhnuH2v2FdFPwY53suJF-87DS5IhyCfepB9gPXg":true,"jJKZt0vsLWnO3HW0PeKVs7UPc7ywwTsVyfujcYU-Ftc":true,"MiKis2HRSnIYK9RNFZCYVDA8m_YWK_GR_c5_qZIAGJg":true,"BUhlISiq8Js05DYyLdqgzjkuWE-7dHkSmQF69RrjPt0":true,"pWP1aV1ljjrhLOcNm1zLeCn6nuNdmRgc0mPWckTKLuE":true,"kXicPE0rdovaUH0rAyT4LK_W7eSUMxNlyveVeOh9-Hw":true,"-Ev1ZFe_TE-ERrNV5b-2UwMuBVXKGdlxoG2SAIxb_Wk":false,"6UBXZRSA6MWZv4lSJulbmBOoc2wJhcwHOOfSN5Pjyzs":false,"mdpV1O8Z97VFLNBp2PUAIpTA-O7v8U8pUuz51KgasC0":true,"LU9r3O0CybN2LAv0MZfXA5Vr_nFxohw-HtBF-MkV7as":false,"tsY9TErbrLsjn7kIgxOGwkV9aIIrnHwYHyZJV2kkejE":false,"RN_1eiX24BVfywQPXvSftRuh-iHwpDgqlj4hfo5SooE":true,"3SUvw4ndIEIFm-eGuHZDDG0TON-xvN1aCWWeBVuDqc8":true,"6n5OyDxawTVrswFy3eb7e-mDDD225T422w3bneHmpM8":false,"38w23-xRm79BheJHrqQpCQZl4JkanE4U_CVqIcCiGTY":true,"rylafD9hDqPx8jBCJJnP_-5rW5AJcqFrNBxgs1m18VU":true,"DQyn8OmHxgdncuvLvsf7OORsluPzdfmZ3MH9vY6bJ7s":true,"z9zXm1-VgyhH9eENr8AZfw-mir5Xd2WyEopzcJVjvrI":true,"rwUNZv7fKMwHM1M8lzEYEy3fMyV1K4B8iz6L4NA52dw":true,"rhSZFxime6CdzGa_45VEu3CAm4oMEdiQXWvS-WtM0xE":true,"H46EsnokWnZ5PWcAuRYlVqigUFYJM8cmn9BbsMZjgtE":true,"QSEIa-gNEgaqq41AFIP1xppfrDeRjSxFXRFhI20DPx8":true,"fCXRcS8HYPJqCqFOH7ap3EOb-UU_MSKap8TE1q67tlA":true,"44t9jiY4pkU4OxG99ptv_a8SAVbQiWT1O1jKmK0BtdY":true,"vJ7_T004nLGMcMXnSQ9jfakXscmMs2ORD1CUj4ZVwIU":true,"xgSBAGdsXv-8jHD7ZE1pqvKD8d9jOYwAUN2RI0dvjrs":true,"-tSi0UL7yot39cFzJtU320oPPx3ZAwbcvHqKBCtTujI":true,"Dwi5np69IlMMB95g-Hr6bNGmTDsqe9VFzmS0QLfWFIw":true,"3MKjavsc6XAxsLWUzJwZ07Ueg8krHqLzhjmUIwjyIQI":true,"_mG9vjuAIwFtJaOdO-RBvaLVWVphVLSK1DKbHG7ALJY":true,"lrDLOyKF5HvBH5d6dqRIO4gaw3sYYtn-cbTCDvOTTq0":true,"o3BEEV1mL3FsynKT7MQD4pQ2phRskMEywMwwbe10AnY":true,"4ihPfXaNZ1Avoga7BDPSlA9Gp3G126W-n3AVWVWKde8":true,"cCobQVKco9LlnXW3SpRvkoYrFvK_Xy7i9WyFyfQnyIQ":true,"s-sss5B33jkcfqqlNbTF2A7b-NiFME83HOtMmP5vmFs":true,"2k9hziOdf8qqORJvqXvh6m-z68gxM9zAYI0RaznfM7w":false,"NdMHMHPLyuKZrnPnh6f800BrEhEhnEfvYdMdMPZeN_k":true,"iGeXf_cItJejRnRiDu4UAbo5F1wSbOsKf0HZLSlIedk":true,"wTsncEaygOxocM8i9tTkMtLkp_RTYzzgqfYGTnF_4Uw":true,"HRRJR9gYiUYI6KQqqtLy-b94nvBOVd3d2PqAE4Bk9pA":true,"_NfLpajWdfKoTbOmbaoBKFM-WyMGBE4uy-nKtZZ8zCY":true,"M8ZXfiiyAtqjMfJ7kGltLIhw3Pi-fctkvysSDANbtnI":true,"ZcVGV9DcxbuDKhet-1iZls8GSHGeXUTfjI1jImywTSc":true,"lmSk1n5vTZ-NUk3Vue4-cKkx_ndwKUp5h6UIm6zM2AI":true,"XNo7R4pZqi7ob4ssDoiPkt9RrfexRcuZ3D4QhIAf_3w":true,"-jC66mP-ywGIPkN2T_i2PoOo9BO1U5oah-NGlaYDXEA":true,"KHYKSQxMNtqu5x_tl8rxzSEwvH5WK5gO5NNWmGoWDHs":true,"HR_1gYuTm-wH2EGMbf8GL339RnGq9nK_OvWdBrmdsjE":true,"xaDh80nfT9VcVjI3w138_hTwdWyxIUbba-qsWuUoMzE":true,"rjgTqVKOyWmgToPuDPyVGxjnZ0vKufkXuXVhG10s4Bk":true,"UizPRE1nj7Xj5KNOYFZfOTqvMi96zAMq7LBjWT0hY88":true,"vMVeIZgIweRRMop4zwsdRiB2zpJ3biin_D2YFEDT5Ig":true,"e1Fa2Df2ehQ_oQuGPknpOCQlvi_Jz6u5XcpU2LXf2sQ":true,"_9CQe8QbNq2bJR_ojIvrZN88lpZaj2rrA_s5uCLDp1E":true,"U8n1B8LJeYynVe3rkXRTa1j3UaQiytimMbVCnpfFRyw":true,"-VrjxZjXb_1h5oZBHoT_jJb7tf94wP3Qn1Oc2yc8dUQ":true,"e1-8bUv99QpL9AW9I9W-hl--qs6AequJEfQTO5TW2WA":true,"aIzFj87wcEzPgpeiyJxpxoltmiaMONl7Fh3mB66XmEs":true,"yaVL1wjTsHeS2HaIeXLk7wGtIxc5VTpSTTvQS-VIFBM":false,"lZ_ievoWUGpMymXz7DqJZB2sHGGLJbmcAia8Hl7qdEs":true,"xOCLVYIsrzzh8QTViasttbfiXBETiTKckyckiJiWsjM":false,"GhNUPO9JOnz9caBHi4xffShvOi-8_UcPPA399aEtVUs":false,"LKsL6zi1p26GzN5EJtmLC79nAf9EUpA8obHd2sVm6j4":true,"VjFp4bF-p3LC_rCMCtnoaNNrruyJC9XWofnKy2JaPYE":true,"-5vyAH-wejQJx-cEEA-MUu98xNg2Ma5Ke0_hA-xc8SY":true,"wPPikUDqRdlkXVg_PYXEaVCrIGwQ39KR2y82j0jqqZI":true,"asQ2s97aXgFy7sj5bSOygcFvozALaIScLk8voVEM_Ac":true,"Ryl7EMxQRimi2ATXkKK94ybUFooXG_k3oO6aPgjPAcM":true,"1ukoDaUnsn0_lWEc6nipkbOhQPgYMjdNTXm347Alncc":true,"KfTDigi9WohH-bdDqDSb_BACYM2oEm8zpDjQQslRH7M":true,"hFFkrgRq7j9ttaAiutcC5l8zJGurzRiKkxj8ty6cGmc":true,"s0KtHEwtZvMNhysmJe9dBwl_bugn201P_zoGH6lC-DM":true,"ir2AJDfMsZ5EbwQdJDn1zQf_-EunF5Dx47anNMz3XPo":true,"rNquq11tS5Ozaow15TuPm-enreTSD0gM7dzm5SBRE-A":true,"D0DT4GB71afEQcybpsrhVM1bZ943VZxdM8JhclM6emE":true,"TOAwCpm8DhT4Iy5feUTZo0B9IBzthp-TpH3AxRxtVNY":true,"36B7rtRhZgZeUh3xtRged9zOq2jEDpN6BOthbQsyImU":true,"zR6LGPRX1MZb6EWq-bDLJ_5nOhwQ1A2PoylQkyT6OjY":false,"r0nGjZh97Ia5CKE64Z0mtD7dKMPKEUxDNqd9gNA2wsQ":false,"39NlZbYnB2ddMCLoahD8wY68soXnCaJ5B99Xw0_dQm4":true,"PTO2O9f4nQZ7LzZrQrPvclB_B4oJnRhKmH4XMsDZw88":true,"oPuAns-V4xc-2otdhGqTHzC6BQSHlkP-kxQnrdS8_D0":false,"c8dQuk1xmXyifTnP_9JWXUUe9d9sjLzVRdNwq1lcWvE":true,"mtfADJnqTkvCcntyza3-4uVkWiy2Yk7uhQTam_qG1rI":true,"rn6zzZfaIPdSQXOpJNCzxFYvnaKQKvzSklG3gB7knlM":true,"11aOMqb1bP-C6b7uTDX5OCOt7YZa1JBnAXCDCKmMPek":true,"JlLWimdruUSyB1ok36EitpZAICIjO6VofC56SUMGbds":true,"wlxP467KtG6l194kPuSAW6g2d1KerxiRFuqWQ4wQTg0":true,"Zp_NiTdudimOT1_b017tfhhOUgwE5L6FGv2CHqd0w1g":true,"ULbpCf58KmOkguXWo0Xk-94KITxjDyrWmYxp-DO4ZjE":true,"ZZm53IT8a-PGlSgBdwCrbAxXAlvUgyNG6W_zKH_6JZQ":true,"EZiyXS7qu-Bjygvw1EseBYOmGa-3RXh89gnHfk2ShEE":true,"CDjrohrVCImpequgbzk9k-e7euARa0IG4NBr0InagJk":true,"Kfk_gkuu_-aho4RuBwjyc_3JqF1XuV8PeI2VpRLkpvs":true,"1YkDZS7BAbRv4hrKvLcCZtL8DCeZQth8oZG2kUt4WOk":true,"7KeQ-ZUctkYgMuAEjGZmHxiHNVguI_BaK_5TwfOGbSY":true,"CMQcDrVzRFUS2zwyKmvg2KrtEhpbTuNVYp0gQfrE_dI":true,"lG0DTLDgLXDv44E27SxiAdPR3lgqT63h0AgZtsNwBpM":false,"ehqNMpBNiPgU_GjI5x2mDyhoiUJ3LrNQaBHdMuF6Ahk":true,"yESj6fDJa4HGYxmXKHJZrSFmmGr5s76PFALWPiQNIQw":true,"4d6d1d5vH6A902X8QW1X8iOxRC36WwEuxf2behLjo28":true,"7qSe_S4wMi68IuaGZ_MfpDoywsttYI6jOME_HbpFkHU":true,"LQfJUgVQgryhTsdk21XIC2PfT2xgDp-XQwF0RyCtrKI":true,"i_m7-uDjyLMiKoNHQ3hRuxiW5Tf6URzH3_LhQnrGNCg":true,"oJ_4EluOnpTStWaDGmNU4U9aFAQU05njWwA1pDvC48A":true,"IhlYHN4QgHIjhfONq2esRp4SVPGJA0ekx5ovVNZPuIA":true,"5q8XrD3mVbwT4nneaoPAPP5YMkj7bgsAxcUEHjRhtYc":true,"hxh9B4C_otZ4fnvkarp5ZWm3qCix4e6BDBD9KAvFsws":true,"0IeCJxaL7mM5mHbWu8uUHPWiV18cf7fF2jSmRLk_Mmk":true,"Zr4Qeb0Z-3pSnZmX7GrdorlkZJludhUqYNgNu6mKSf0":true,"xUzYs3nVYSO69wDz_BdgX-zEh2i4N2bQSo-wHMDu57M":true,"WbMUJsvnr8TJcFYJwQO8lmAqQoWlR8drgy8ah7Kk4fc":true,"Uk9Gghnu1x9X8DmxhwbxI0L_qn7opg2hX5OTUAf0Vis":true,"FCCOQQV0dn_CawLEn5qLlX1VjcGKe1OLRRBcPrWrWsY":true,"r1elQ_PmDM6z_U3MMfed1eQt2gMOaXw6rqK0DKATuog":true,"KNhzEsFRVyxr_k1xbhgGkurGGsKsAugxv5xLkjG3X9U":true,"IRxpZl86amrl5t92ORaFtF4qhx2S-o4sJ_rkg4b2GqI":true,"Pvi2_oz9JBinyPFE0KaZ_G-_pVceax_-kvleGwetEwA":true,"BvAX9KSdq1aBkJrYsD3ZmDiMqTqFnbXTKmG5pYBelpg":true,"UKFN3BkYUNa_jjtfyabm5SRvTVr1w7aMgvRfFlWuU0k":true,"R4r7vFceewVn2BMU6MHaUf1g8s3SNCDSjLB-k507yoA":true,"rxnmqBqtzcFVXUhF2cmpI2YYb-dKI_8hoTWK4KmPiRw":true,"M5DM8PWn82l4KyIaADkV9X66gWIr80EhS7xE-EmJwv0":true,"HaLWXRFtAb0Pcn11IIcdyxYgndU27xzw0zHgZ5m6z_0":true,"ClpuhP74mYQ7GPsF9cK1NCYw71205feGbTGWWeuloDU":true,"8uxAn49Sx8tQhpQ5M11KCf1zdhAqn2EI6cJIcqPUYck":true,"VHo7jzPn5uXVu6rTlvAmwudYU-sIbjOGcpAlvrAONWw":true,"7SUwhq0UJ0L3IQ9-18Gq2y_9b1YjqVgoBaeEDH25kGQ":true,"TiqTa-6ZpHhg45hCNG3hDRZk0PXF6CmVGCkz5cZtY5M":true,"4rXreLFr5AXZb8uPRSCzwn_V_3UMn_NjTb5adBkywT0":true,"3ft5ETALNMABKHMpwKPdiSsLtvid33ZaTb7gGApsWlU":true,"LEskuYHB5OPb3xaJb09e3YHP4ihVq9mGIyAoIS0G1cI":true,"8joDQdmBusv2BoJQr-oXFXDXi90ugMXxfpRzJScRDmI":true,"E0pVaV7naECO_c5MOYIGcsoXqPKzUKAAnnLS55A8jFs":true,"MJVperuSfFAWwBUAvEEseOYflg_zVsHRB3iKw-nJEjk":true,"FXeWKiHz_9SSQ9mibse-rJ5e_nuapkr7JC8DHHg0lFo":true,"pyr3YpCyA9Zb2mnf1-CCcKlYdYvhf1xpUq8A7at7pig":true,"nybAbTj6RkOY2A5_SZl20z0QKbTi5Bu8plAbEM4CeR8":true,"fkfiDZbMsLAStRex4slt8QY0rfYd0jjFv-RLBtY0q0g":true,"wFFBZ85AOzfid5ykvdwfAKMitx4MW2NOwwqsIdTRqWY":true,"ulx3AqOecjEN7Qh_-dSxvTr5zBU4staDu5YCuAQNsbs":true,"6EprAqA9Z5NHhOXqK9_Mimb_LMBtg7ufhmjvMHKRfx8":true,"ublPFv3tYcsT8KrdzMm0bbE5TVB-FHP1BGnj-sqjjxg":true,"R-NoK_O31E-qhlGhVgXlde01CHaJ41NscC6CSe-pAvQ":true,"fpifuIwJgNo3DQ-jj3IVEMYIwlwtiKRVb12ZNl7YfJE":true,"VYMVBHsr_zxaqpV8oLXgWwKOIXTKDDiPJ2QbMyFF-PY":true,"B6-4i2h087wUBOfXNgAKxc3We0ri76i0Q0QlyVMziMs":true,"bnyctwlw_KazNy9NkEsoQCSC01aIKW0IZKrkHWqLDok":true,"JLKwISYyGo77dsR9HjueCXEavPPawZvb-_0zVpOXhMM":true,"uHwdkXQESsuy-y9D1M3GZ_JgEVwDSVu0UQbLsJs2rRY":true,"1IkF7eRInXgwtaFGf-5SmBhtc-YHJU52X23HBJlxpAo":true,"wc28uxF7pBPj5K6EzSvno2OZy4GJcfYw3wba3g0pXik":true,"0d1grzgCL_ZxUzshRsxXCX_ipOaB3YZsQp4o-BNFCfk":true,"g539IyldKz6PCSHw4reigZOWvBt_WW_7GaesumLfxMQ":true,"W7Q9oz4JM1jWlnYF9zZNLKZX3BJIHhhDHUOKjU1Tl4k":true,"SSebI7ViDwH1iRlrlnzWd6bm9D9K7aej7PMbifYkGHA":true,"8-ytG0KQ5nkvZE-RkOA5BL7kAde6P1kPh6MNjzepI54":true,"cFgvkDyepmV-vJ38mObYoFGfZcieC-XLOy0MbBIdOL8":true,"l3-GUKeIf6z7jkGSY0U7YAA6aMHi3rf178bJuFyxxsQ":true,"rKxg20-KprBfsMbwVsrp_YshobNlf5JrPRc2IqPlkDo":true,"VpZgQ9L0QVOzA2wV2HF1pI_Kp1TySNTIJLUcPhS1tXY":false,"DLcRUwkulDQBfcjaoGnQJwbp9gZ0CGoWRj5TEWPfkTM":true,"RpEeAoDVBQrD7zEsIHwBdqtWUHlf46X4dDgzAWkN-ms":true,"idXVkgi6NvKNDv3pAgDVQzeMmhmvvQOWwF95L1GsM6w":false,"awbkeViqceRG-ogDgvrxS1G7f8NGY3w_ysgp-gPFEis":true,"7mB_s-yXbsOTiVchv9EDyMR5eWCAkjJGvqFzx156SLo":true,"ecKnI8zjYhxXPte2idmBPXz-J0igvyrRA74wylsVbUY":false,"8pLs8vDs_iuHbjG42D79obHw-iTgU2p2ghPauV9AfKU":true,"kuE5HpusLTQvn0ITFPFAgpV4CjnvcmyQNAy4jfGcM1Y":true,"B1C2R9iSbmpfcfvj6O0JvFLXn62aYKeztLOWB5HqOgU":false,"Fhf2x3n_Ojy7zxns5sc2GjUtZuq_7owRwFOF0EBS71c":true,"f_chx_m7S8v1uwVKp1c1Fz1B04lfziy39mWMFug1POk":false,"u23d4nMT2kJhdYOutfJ-GFg2f8KwAeu0aXlmHF-L_aU":true,"L7H2M_ORo36Xyw9Ap9aK45bEMARIFNJUAg9zYKeM7lM":true,"jOQCYs-LZdQ3c1an7mZp3Yfr3wrFUBH0CB5l0dfsk8g":true,"WG0qwgdnl7yYNX5a7kEuBezlD2pbSeLI5dBICUB8FbY":false,"GBFHc_6lbs-YMTtSj8fdeqiMjTCTZOwdsGv5F5Shn1E":false,"qABPPDKuEooLXdrdAiW0g5CRxi5B7xBk8MkdiWblAUU":false,"mO8epc9_mSnHc_i4JAPtjT1ExrL7JxbyzhkqFkQid7Q":true,"HyhSWo3q91LRI3eeoEs0-wASTfUaUkg-qUiAKXAlImY":true,"LUu6OlEDBqEtndxJsvCX5qN4NvqBwvK_1dJeWJQnvOY":true,"FsPhXFYPe-SPxtbkldXP0uU7B_jRHKxHi9ofMQVSyMQ":true,"0a81hBZv4MTJWxivDSEvdiw0ve0-AMOAAkTi5cat2t8":true,"hy1n7qUqtlKYH5BDDA7srhKiezDVkE_JuIXrn1HA2r0":true,"zEYzhAI8ZNFjSuu-3k5D0sksj9dQny1Ii88S7w_IFTc":false,"ijW0WVmExJzPA17ysMqGE23RcVpQVA4fyOFbJlcbvvM":true,"0xdHaZkXHqeN-UUPiJ7tGbHWtklWkDb74ZC45Zo7E3M":true,"lZMVSikHI1fbLgSfqv6mQi6Zobq_oyZtow9yQ9xmUWo":true,"vXCEOGCkD6uXyj29LfABUgKnp4qeCECF2Ds11TSE3jA":true,"qsbXyCangDD9btwu-SGD7LPRAsOBIp2di6Yt8HLfrPU":true,"x1mXXdtC_hIsuo02NN4WLnL4_ry7y6Cd4Pd6t-VbjhU":true,"WauBO1G_fFMYeshQozEiycg70rzrdWk8BkmF3i9zjOY":true,"QiU__T0xy6DyIzNz75ggGZYENjIzty03Yt-Ql6kDMJU":true,"t215sid3gVqRbQFZv3CteKDJRd2DjFUo0npGo3CV3aQ":true,"m5nfoN5SBPypwJAb3Sgc9lDGZp7_l6f961hf_zMMcKQ":true,"Hs0L-JrYf3U-aHD-BWRv4bFWHS7OuzgtzdAHEKiGHMI":true,"B9BzcbHdNb9eIkNqGQz3SfeBjNrdVyFwGbuW9at8VRw":true,"huGjA_smraRG0MA9u6U3s-CdL2lIkZ93dg_uAR1q7_k":true,"tKM-sJSm2QH1_mkhGt30LQq3r-jH3tp9Kamh_pGqlis":true,"Ra938t9nRdU1lX2U1JTVDF26i0wOZgZy9rj7kdFknYA":true,"3DkROCvAx-0umI4W6Xlvek1HVISCO6CQjh2h-vb_V4g":false,"X_4F3V41oNdWjWz4Smdl8xXTdGJfJsIhyDiXJIGQCjo":false,"wrIODzCcH9NYBBe_kTB_AVTaBjsQ6oOtZ42vH-l1AwM":false,"L03DaPrGW7GFHkLlwSAdDu1aa-weiXVV9gYSclAFmyI":false,"YtkI6W_0tyXo8fju5_JwXXy5qiOWY0P4Uiq4bXA1jHQ":true,"0J9RP8MwtB6y-Z3oRNgdURHXC2p5u5pm3AGJH4JBPLM":true,"RqBAEgQtmpIwV9SYqxh0eZpil85Hh1ILhVBg8B9FmxA":true,"dW9lKFdg7a-oYmuCry-jbpwPwUbUReQxPgQiKyQn_0s":true,"bzxZvSI-qusnF76xsRsZyasd5PFP-JZS8LrqwCsLP7s":true,"lRF16jduwzYYD_33ihxKqVQ3GsYR-G2Cy7YEUSOh19M":true,"GZCRdoX45gxbmdcRhFNoEwz7vziglfnQ997btRFNU6c":true,"Ag2a6fiN7VEJNcWr8j8i1Nuce3_ORJvR8Wj1DcFqhUM":true,"6wXL1rZY3ZPyv6z32P8qloC91GRsMPhOTpELEXPgMoY":true,"ZlZdtBuaGco7DobKMIBW2G_6dPs7MO-ZvSYW_AYD_TM":true,"tIhJj8NhYHpGlVw173KU5skTUWWmxMjUxNdFQrMHJYg":true,"-ohGP4jBn2IFl8nJQnJSzdnxpKgSMFpSjgFjL_ldaoo":true,"8mLCqIHA1tedq6HmAIyAY0rWGpcSEG5_XUJHXlFhZIU":true,"Uw_16n5Advkqn1inNEqbTBeQcnGGqslP_YNS2oGtF1M":true,"ZSs7OJBzr26RlHAau6Z9BO9O536zcuF2fJfaiM0AQYw":true,"FVkXq0-jESjANcdo0L7NbjUL9zyps4MJAGdyefEyB2U":true,"dkrsTT1Ue_zDDbKTtYQ9qAmV5ULrDP4TRqmmdl2cg4w":true,"U9qSmRRRn2h1F39zUIHbSLP6ZwSfi8UUx5tgrDcUsgo":true,"dJJ7ReZR0LCeiL9A66ByVE0tMTMn-pix7p4ptDEUbyo":false,"tQ4GsOrXJ502wHGZXTaQzt_a1sjUhQVtqPi0XIM2F3k":true,"59XTsljhGhZuKzpYJRl4TlTUz5PQfvLEb5Uf5FGBglM":true}"#;

    let smartweave_validity: IndexMap<String, Value> =
      serde_json::from_str(smartweave).unwrap();
    let arweave = Arweave::new(
      80,
      String::from("arweave.net"),
      String::from("https"),
      ArweaveCache::new(),
    );
    let result = execute_contract(
      String::from("t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y-BPZgKE"),
      Some(838269),
      false,
      false,
      None,
      None,
      &arweave,
    )
    .await
    .unwrap();

    if let ExecuteResult::V8(value, validity) = result {
      let map: IndexMap<String, Value> = validity;
      let keys = map.keys();

      if keys.len() != smartweave_validity.keys().len() {
        panic!("Invalid amount of transactions");
      }

      for i in 0..(keys.len()) {
        let (key, value): (&String, &Value) = map.get_index(i).unwrap();
        let (smartweave_key, smartweave_value): (&String, &Value) =
          smartweave_validity.get_index(i).unwrap();

        if smartweave_key != key {
          panic!(
            "Key index #{} from 3EM {} is different from smartweave {}",
            i, key, smartweave_key
          );
        }
      }
    } else {
      assert!(false);
    }
  }

  #[tokio::test]
  async fn test_execute_wasm() {
    let arweave = Arweave::new(
      80,
      String::from("arweave.net"),
      String::from("https"),
      ArweaveCache::new(),
    );
    let result = execute_contract(
      String::from("KfU_1Uxe3-h2r3tP6ZMfMT-HBFlM887tTFtS-p4edYQ"),
      Some(822062),
      false,
      false,
      None,
      None,
      &arweave,
    )
    .await
    .unwrap();
    if let ExecuteResult::V8(value, validity) = result {
      assert!(!(value.is_null()));
      assert!(value.get("counter").is_some());
      let counter = value.get("counter").unwrap().as_i64().unwrap();
      assert_eq!(counter, 2);
      assert!(validity
        .get("HBHsDDeWrEmAlkg_mFzYjOsEgG3I6j4id_Aqd1fERgA")
        .is_some());
      assert!(validity
        .get("IlAr0h0rl7oI7FesF1Oy-E_a-K6Al4Avc2pu6CEZkog")
        .is_some());
    } else {
      assert!(false);
    }
  }

  #[tokio::test]
  async fn test_execute_javascript() {
    let arweave = Arweave::new(
      80,
      String::from("arweave.net"),
      String::from("https"),
      ArweaveCache::new(),
    );
    let result = execute_contract(
      String::from("t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y-BPZgKE"),
      None,
      false,
      false,
      None,
      None,
      &arweave,
    )
    .await
    .unwrap();
    if let ExecuteResult::V8(value, _validity) = result {
      assert!(!(value.is_null()));
      assert!(value.get("people").is_some());
      assert!(value.get("people").unwrap().is_array());
      let people = value.get("people").unwrap();
      let people_struct: Vec<People> =
        serde_json::from_value(people.to_owned()).unwrap();
      let is_marton_here = people_struct
        .iter()
        .find(|data| data.username == *"martonlederer");
      assert!(is_marton_here.is_some());
    } else {
      assert!(false);
    }
  }
}
