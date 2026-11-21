;; An Activity that reaches the server through nothing but the host functions,
;; made into a component against `wit/plugin.wit`'s `activity` world at test
;; time by `wit-component`, as `activity.wat` is.
;;
;; `run` launches one session under `verkstead/implementing`, as the
;; Implementation role, expecting a commit (`committed-since(0)`) and ending
;; with its work; waits on it; reads the git facts of the Worktree's repo,
;; whose name is the first field of the facts it was handed; and answers
;; `committed` where the branch holds commits past its base and `nothing` where
;; it holds none. A launch refused answers `refused`, and git facts refused
;; `unread`, neither of which a block declares.
;;
;; The canonical ABI, as these calls lower: `launch`'s eleven flat parameters
;; and a pointer to where its `result<session, string>` goes (the tag at 0, the
;; session at 8); `wait` a session and a pointer to where its `waited` goes
;; (sixteen bytes, at 144); `git-facts` a string and a pointer to its
;; `result<branch-facts, string>` (the tag at 0, the commits at 12).
(module
  (import "verkstead:plugin/host@1.0.0" "launch"
    (func $launch
      (param i32 i32 i32 i32 i32 i32 i32 i64 i32 i32 i32 i32)))
  (import "verkstead:plugin/host@1.0.0" "wait"
    (func $wait (param i64 i32)))
  (import "verkstead:plugin/host@1.0.0" "git-facts"
    (func $git_facts (param i32 i32 i32)))

  (memory (export "memory") 1)
  (data (i32.const 256) "verkstead/implementing")
  (data (i32.const 288) "implementation")
  (data (i32.const 320) "The fixture Activity asks for one commit, then for verkstead done.")
  (data (i32.const 448) "committed")
  (data (i32.const 464) "nothing")
  (data (i32.const 480) "refused")
  (data (i32.const 496) "unread")
  (global $next (mut i32) (i32.const 1024))

  ;; A bump allocator that grows the memory as it goes: the facts carry the
  ;; whole Timeline, which is more than one page soon enough.
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

  ;; The outcome at `$at`, `$length` long, as the pointer to the pair `run`
  ;; answers with.
  (func $answer (param $at i32) (param $length i32) (result i32)
    (i32.store (i32.const 16) (local.get $at))
    (i32.store (i32.const 20) (local.get $length))
    (i32.const 16))

  (func (export "run") (param $facts i32) (result i32)
    ;; launch(skill, role, prompt, committed-since(0), with-its-work)
    (call $launch
      (i32.const 256) (i32.const 22)
      (i32.const 288) (i32.const 14)
      (i32.const 320) (i32.const 66)
      (i32.const 1) (i64.const 0) (i32.const 0) (i32.const 0)
      (i32.const 0)
      (i32.const 128))
    (if (i32.load8_u (i32.const 128))
      (then (return (call $answer (i32.const 480) (i32.const 7)))))

    ;; Whatever the wait came to, the branch is what says how it ended.
    (call $wait (i64.load (i32.const 136)) (i32.const 144))

    (call $git_facts
      (i32.load (local.get $facts))
      (i32.load (i32.add (local.get $facts) (i32.const 4)))
      (i32.const 160))
    (if (i32.load8_u (i32.const 160))
      (then (return (call $answer (i32.const 496) (i32.const 6)))))

    (if (result i32) (i32.load (i32.const 172))
      (then (call $answer (i32.const 448) (i32.const 9)))
      (else (call $answer (i32.const 464) (i32.const 7))))))
