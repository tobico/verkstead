;; The least that is an Activity: a core module whose `run` answers `committed`
;; whatever it is handed, made into a component against `wit/plugin.wit`'s
;; `activity` world at test time by `wit-component`, so no suite needs a wasm
;; toolchain. `run`'s one argument is past what is passed flat, so it arrives
;; as a pointer, and its string goes back as a pointer to the pair.
(module
  (memory (export "memory") 1)
  (data (i32.const 64) "committed")
  (global $next (mut i32) (i32.const 1024))

  (func (export "cabi_realloc")
    (param $old i32) (param $old_size i32) (param $align i32) (param $size i32)
    (result i32)
    (local $at i32)
    (local $end i32)
    (local.set $at
      (i32.and
        (i32.add (global.get $next) (i32.sub (local.get $align) (i32.const 1)))
        (i32.sub (i32.const 0) (local.get $align))))
    (local.set $end (i32.add (local.get $at) (local.get $size)))
    ;; Grown as it goes: facts carry the whole Timeline.
    (if (i32.gt_u (local.get $end) (i32.mul (memory.size) (i32.const 65536)))
      (then
        (drop (memory.grow
          (i32.add
            (i32.div_u
              (i32.sub (local.get $end) (i32.mul (memory.size) (i32.const 65536)))
              (i32.const 65536))
            (i32.const 1))))))
    (global.set $next (local.get $end))
    (local.get $at))

  (func (export "run") (param $facts i32) (result i32)
    (i32.store (i32.const 16) (i32.const 64))
    (i32.store (i32.const 20) (i32.const 9))
    (i32.const 16)))
