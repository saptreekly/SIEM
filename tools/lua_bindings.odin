package lua_bindings

// Bindings for LuaJIT 5.1
foreign import luajit "system:luajit-5.1"

@(default_calling_convention="c")
foreign luajit {
    luaL_newstate :: proc() -> rawptr ---
    luaL_openlibs :: proc(L: rawptr) ---
    luaL_loadstring :: proc(L: rawptr, s: cstring) -> i32 ---
    lua_pcall :: proc(L: rawptr, nargs: i32, nresults: i32, errfunc: i32) -> i32 ---
    lua_close :: proc(L: rawptr) ---
    
    // For retrieving data from Lua
    lua_getglobal :: proc(L: rawptr, name: cstring) ---
    lua_tostring :: proc(L: rawptr, idx: i32) -> cstring ---
}
