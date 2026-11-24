;; An Activity that starts roadmap stages through the host and reads one back
;; until it settles, putting what it was told on the Timeline as Notices, in
;; order. Made into a component against `wit/plugin.wit`'s `activity` world at
;; test time by `wit-component`, as `activity.wat` is.
;;
;; `run`, of the `rate-limiting` roadmap, every stage cut from the Worktree's
;; branch — the third field of the facts it was handed — and stacked, so no
;; fetch is made:
;;
;;   1. starts stage 01, *Count the requests*, wanting no platform;
;;   2. starts stage 02, *Refuse the rest*, wanting `windows` — which a test
;;      may rewrite, with its length, to a platform this device is not;
;;   3. reads stage 01's state until it has settled;
;;   4. starts stage 02 again;
;;
;; and answers `started`. A start's Notice is the branch it started on, or the
;; sentence it was refused in; the settled state's is its word. Stage 01 not
;; starting answers `unstarted` there and then, and its state refused answers
;; `unread`, after the sentence.
;;
;; The canonical ABI, as these calls lower: `start-conversation` takes the
;; stage's seven strings (fourteen) and its bool, and a pointer to where its
;; answer goes — twenty-four bytes at 128: the result's tag at 0, then a start's
;; conversation at 8 and its branch at 16 and 20, or the refusal's tag at 8
;; and its sentence at 12 and 16. `conversation-state` takes the conversation
;; and a pointer to its answer — at 192: the tag at 0, then the state's word
;; (or the refusal) at 4 and 8, and whether it settled at 12.
(module
  (import "verkstead:plugin/host@1.0.0" "start-conversation"
    (func $start
      (param i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)))
  (import "verkstead:plugin/host@1.0.0" "conversation-state"
    (func $state (param i64 i32)))
  (import "verkstead:plugin/host@1.0.0" "notice"
    (func $notice (param i32 i32)))

  (memory (export "memory") 1)
  (data (i32.const 256) "rate-limiting")
  (data (i32.const 272) "01")
  (data (i32.const 276) "02")
  (data (i32.const 288) "Count the requests")
  (data (i32.const 320) "Refuse the rest")
  (data (i32.const 352) "docs/roadmaps/rate-limiting/01-counter.md")
  (data (i32.const 400) "docs/roadmaps/rate-limiting/02-refusing.md")
  (data (i32.const 448) "# 01. Count the requests")
  (data (i32.const 480) "# 02. Refuse the rest")
  (data (i32.const 512) "windows")
  (data (i32.const 528) "started")
  (data (i32.const 544) "unstarted")
  (data (i32.const 560) "unread")
  (global $next (mut i32) (i32.const 1024))
  (global $branch (mut i32) (i32.const 0))
  (global $branch_length (mut i32) (i32.const 0))

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

  ;; The start's answer at 128 as a Notice, answering the conversation it
  ;; started, or -1 where it did not.
  (func $noticed_start (result i64)
    (if (i32.load8_u (i32.const 128))
      (then
        (call $notice (i32.load (i32.const 140)) (i32.load (i32.const 144)))
        (return (i64.const -1))))
    (call $notice (i32.load (i32.const 144)) (i32.load (i32.const 148)))
    (i64.load (i32.const 136)))

  ;; Stage 01.
  (func $first (result i64)
    (call $start
      (i32.const 256) (i32.const 13)
      (i32.const 272) (i32.const 2)
      (i32.const 288) (i32.const 18)
      (i32.const 352) (i32.const 41)
      (i32.const 448) (i32.const 24)
      (global.get $branch) (global.get $branch_length)
      (i32.const 0) (i32.const 0)
      (i32.const 1)
      (i32.const 128))
    (call $noticed_start))

  ;; Stage 02.
  (func $second (result i64)
    (call $start
      (i32.const 256) (i32.const 13)
      (i32.const 276) (i32.const 2)
      (i32.const 320) (i32.const 15)
      (i32.const 400) (i32.const 42)
      (i32.const 480) (i32.const 21)
      (global.get $branch) (global.get $branch_length)
      (i32.const 512) (i32.const 7) ;; the platform's length
      (i32.const 1)
      (i32.const 128))
    (call $noticed_start))

  ;; The pointer to the pair `run` answers: the word at `$at`, `$length` long.
  (func $answer (param $at i32) (param $length i32) (result i32)
    (i32.store (i32.const 16) (local.get $at))
    (i32.store (i32.const 20) (local.get $length))
    (i32.const 16))

  (func (export "run") (param $facts i32) (result i32)
    (local $one i64)
    (global.set $branch (i32.load (i32.add (local.get $facts) (i32.const 16))))
    (global.set $branch_length (i32.load (i32.add (local.get $facts) (i32.const 20))))

    ;; 1. and 2.
    (local.set $one (call $first))
    (drop (call $second))

    (if (i64.lt_s (local.get $one) (i64.const 0))
      (then (return (call $answer (i32.const 544) (i32.const 9)))))

    ;; 3. stage 01's state, until it has settled
    (block $settled
      (loop $reading
        (call $state (local.get $one) (i32.const 192))
        (if (i32.load8_u (i32.const 192))
          (then
            (call $notice (i32.load (i32.const 196)) (i32.load (i32.const 200)))
            (return (call $answer (i32.const 560) (i32.const 6)))))
        (br_if $settled (i32.load8_u (i32.const 204)))
        (br $reading)))
    (call $notice (i32.load (i32.const 196)) (i32.load (i32.const 200)))

    ;; 4.
    (drop (call $second))

    (call $answer (i32.const 528) (i32.const 7))))
