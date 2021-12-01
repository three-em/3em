const std = @import("std");
const json = std.json;

var LEN: usize = 0;

export fn get_len() usize {
    return LEN;
}

export fn _alloc(len: usize) usize {
    var buf = std.heap.page_allocator.alloc(u8, len) catch |err| return 0;
    return @ptrToInt(buf.ptr);
}

const State = struct {
    counter: usize,
};

fn neat_handle(state: State) State {
    return State{
        .counter = state.counter + 1,
    };
}

export fn handle(
    state_ptr: [*]u8,
    state_size: usize,
    action_ptr: [*]u8,
    action_size: usize,
) [*]u8 {
    const state_slice = state_ptr[0..state_size];
    const action_slice = action_ptr[0..action_size];

    var stream = json.TokenStream.init(state_slice);
    var state = json.parse(State, &stream, .{}) catch unreachable;
    var result = neat_handle(state);

    var alloc = std.heap.page_allocator;
    var string = std.ArrayList(u8).init(alloc);

    json.stringify(result, .{}, string.writer()) catch unreachable;

    LEN = string.items.len;

    return string.items.ptr;
}
