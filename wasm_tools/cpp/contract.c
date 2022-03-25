#include "three_em.h"

cJSON *
handler (cJSON * state, cJSON * action)
{
  cJSON *result = NULL;
  cJSON *counter = NULL;

  counter = cJSON_GetObjectItemCaseSensitive (state, "counter");
  result = cJSON_CreateObject ();
  cJSON_AddNumberToObject (result, "counter", counter->valueint + 1);
  return result;
}

MAKE_CONTRACT (handler)
