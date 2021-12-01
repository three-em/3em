@_cdecl("handle")
func handle(
    state_ptr: UnsafePointer<Uint8>,
    state_len: Int32,
    action_ptr: UnsafePointer<Uint8>,
    action_len: Int32,
) -> UnsafePointer<Uint8> {
    let state = Data(bytes: state_ptr, count: Int(state_len))
    let action = Data(bytes: action_ptr, count: Int(action_len))
    let result = handle(state: state, action: action)
    
}