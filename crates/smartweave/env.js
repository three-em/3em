(function (window) {
    window.env = {
     toObject: () => Deno.core.opSync("op_three_em_env")
    };
})(this);
