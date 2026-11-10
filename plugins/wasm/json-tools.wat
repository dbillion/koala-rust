;; json-tools.wat - JSON manipulation tools
;; Exports: get_manifest, get_tools, set_manifest, set_tools, allocate, deallocate, memory, call_tool

(module
  (memory 1)
  (export "memory" (memory 0))

  ;; Global variables to store JSON pointers and lengths
  (global $manifest_ptr (mut i32) (i32.const 0))
  (global $manifest_len (mut i32) (i32.const 0))
  (global $tools_ptr (mut i32) (i32.const 0))
  (global $tools_len (mut i32) (i32.const 0))

  ;; Simple allocator (bump pointer)
  (global $heap_ptr (mut i32) (i32.const 100))

  ;; allocate(size) -> ptr
  (func $allocate (param $size i32) (result i32)
    (local $ptr i32)
    (global.get $heap_ptr)
    (local.set $ptr)
    (global.get $heap_ptr)
    (local.get $size)
    (i32.add)
    (global.set $heap_ptr)
    (local.get $ptr)
  )
  (export "allocate" (func $allocate))

  ;; deallocate(ptr, size) -> void (no-op for bump allocator)
  (func $deallocate (param $ptr i32) (param $size i32) (result i32)
    (i32.const 0)
  )
  (export "deallocate" (func $deallocate))

  ;; set_manifest(ptr, len) -> void - called by host after allocate+write
  (func $set_manifest (param $ptr i32) (param $len i32)
    (global.set $manifest_ptr (local.get $ptr))
    (global.set $manifest_len (local.get $len))
  )
  (export "set_manifest" (func $set_manifest))

  ;; set_tools(ptr, len) -> void - called by host after allocate+write
  (func $set_tools (param $ptr i32) (param $len i32)
    (global.set $tools_ptr (local.get $ptr))
    (global.set $tools_len (local.get $len))
  )
  (export "set_tools" (func $set_tools))

  ;; get_manifest() -> (ptr, len)
  (func $get_manifest (result i32 i32)
    (global.get $manifest_ptr)
    (global.get $manifest_len)
  )
  (export "get_manifest" (func $get_manifest))

  ;; get_tools() -> (ptr, len)
  (func $get_tools (result i32 i32)
    (global.get $tools_ptr)
    (global.get $tools_len)
  )
  (export "get_tools" (func $get_tools))

  ;; call_tool(name_ptr, name_len, args_ptr, args_len, out_ptr, out_len) -> i32
  (func $call_tool (param $name_ptr i32) (param $name_len i32) (param $args_ptr i32) (param $args_len i32) (param $out_ptr i32) (param $out_len i32) (result i32)
    (i32.const 0)
  )
  (export "call_tool" (func $call_tool))
)