/* tslint:disable */
/* eslint-disable */

/* auto-generated by NAPI-RS */

export interface ExecuteContractResult {
  state: any
  validity: Record<string, any>
}
export interface ExecuteConfig {
  host: string
  port: number
  protocol: string
}
export interface Tag {
  name: string
  value: string
}
export interface Block {
  height: string
  indepHash: string
  timestamp: string
}
export interface SimulateInput {
  id: string
  owner: string
  quantity: string
  reward: string
  target?: string | undefined | null
  tags: Array<Tag>
  block?: Block | undefined | null
  input: any
}
export function simulateContract(contractId: string, interactions: Array<SimulateInput>, contractInitState?: string | undefined | null, maybeConfig?: ExecuteConfig | undefined | null, maybeCache?: boolean | undefined | null, maybeBundledContract?: boolean | undefined | null): Promise<ExecuteContractResult>
export function executeContract(tx: string, maybeHeight?: number | undefined | null, maybeConfig?: ExecuteConfig | undefined | null): Promise<ExecuteContractResult>
