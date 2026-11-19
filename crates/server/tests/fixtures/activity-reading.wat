;; An Activity that reads the Repo through the host and puts every answer on
;; the Timeline as a Notice, in order, so a test reads back what it was told.
;; Made into a component against `wit/plugin.wit`'s `activity` world at test
;; time by `wit-component`, as `activity.wat` is.
;;
;; `run` asks, of the Worktree's repo — whose name is the first field of the
;; facts it was handed — unless a step says otherwise:
;;
;;   1. `.tasks/TODO.md` at the Worktree's HEAD;
;;   2. the entries of `.tasks` at the same, one Notice apiece;
;;   3. `note.md` at the branch `elsewhere`;
;;   4. `x` in a repo called `nowhere`;
;;   5. `../outside` at HEAD;
;;   6. `x` at the branch `no-such-branch`;
;;   7. `missing.md` at HEAD;
;;
;; and answers `read`. A file's Notice is its contents, a refusal's is the
;; sentence, and nothing there is `none`.
;;
;; The canonical ABI, as these calls lower: each read takes the repo (two),
;; the revision (its tag, then a name's two, zero for `head`) and the path
;; (two), and a pointer to where its answer goes — sixteen bytes at 128: the
;; result's tag at 0, then an error's string at 4 and 8, or the option's tag
;; at 4 and its list at 8 and 12.
(module
  (import "verkstead:plugin/host@1.0.0" "read-file"
    (func $read_file (param i32 i32 i32 i32 i32 i32 i32 i32)))
  (import "verkstead:plugin/host@1.0.0" "list-directory"
    (func $list_directory (param i32 i32 i32 i32 i32 i32 i32 i32)))
  (import "verkstead:plugin/host@1.0.0" "notice"
    (func $notice (param i32 i32)))

  (memory (export "memory") 1)
  (data (i32.const 256) ".tasks/TODO.md")
  (data (i32.const 272) ".tasks")
  (data (i32.const 288) "elsewhere")
  (data (i32.const 304) "note.md")
  (data (i32.const 320) "nowhere")
  (data (i32.const 336) "x")
  (data (i32.const 352) "../outside")
  (data (i32.const 368) "no-such-branch")
  (data (i32.const 384) "missing.md")
  (data (i32.const 400) "none")
  (data (i32.const 416) "read")
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

  ;; The refusal, or `none`, at 128 as a Notice: true where it was one of
  ;; them, and the answer is not there to read.
  (func $refused_or_none (result i32)
    (if (i32.load8_u (i32.const 128))
      (then
        (call $notice (i32.load (i32.const 132)) (i32.load (i32.const 136)))
        (return (i32.const 1))))
    (if (i32.eqz (i32.load8_u (i32.const 132)))
      (then
        (call $notice (i32.const 400) (i32.const 4))
        (return (i32.const 1))))
    (i32.const 0))

  ;; A file read's answer as a Notice.
  (func $noticed_file
    (if (i32.eqz (call $refused_or_none))
      (then
        (call $notice (i32.load (i32.const 136)) (i32.load (i32.const 140))))))

  ;; A listing's answer as a Notice per name.
  (func $noticed_listing
    (local $at i32)
    (local $end i32)
    (if (call $refused_or_none) (then (return)))
    (local.set $at (i32.load (i32.const 136)))
    (local.set $end
      (i32.add (local.get $at) (i32.mul (i32.load (i32.const 140)) (i32.const 8))))
    (block $done
      (loop $each
        (br_if $done (i32.ge_u (local.get $at) (local.get $end)))
        (call $notice (i32.load (local.get $at)) (i32.load (i32.add (local.get $at) (i32.const 4))))
        (local.set $at (i32.add (local.get $at) (i32.const 8)))
        (br $each))))

  (func (export "run") (param $facts i32) (result i32)
    (local $repo i32)
    (local $repo_length i32)
    (local.set $repo (i32.load (local.get $facts)))
    (local.set $repo_length (i32.load (i32.add (local.get $facts) (i32.const 4))))

    ;; 1. `.tasks/TODO.md` at HEAD
    (call $read_file (local.get $repo) (local.get $repo_length)
      (i32.const 0) (i32.const 0) (i32.const 0)
      (i32.const 256) (i32.const 14) (i32.const 128))
    (call $noticed_file)

    ;; 2. `.tasks` at HEAD
    (call $list_directory (local.get $repo) (local.get $repo_length)
      (i32.const 0) (i32.const 0) (i32.const 0)
      (i32.const 272) (i32.const 6) (i32.const 128))
    (call $noticed_listing)

    ;; 3. `note.md` at `elsewhere`
    (call $read_file (local.get $repo) (local.get $repo_length)
      (i32.const 1) (i32.const 288) (i32.const 9)
      (i32.const 304) (i32.const 7) (i32.const 128))
    (call $noticed_file)

    ;; 4. `x` in `nowhere`
    (call $read_file (i32.const 320) (i32.const 7)
      (i32.const 0) (i32.const 0) (i32.const 0)
      (i32.const 336) (i32.const 1) (i32.const 128))
    (call $noticed_file)

    ;; 5. `../outside` at HEAD
    (call $read_file (local.get $repo) (local.get $repo_length)
      (i32.const 0) (i32.const 0) (i32.const 0)
      (i32.const 352) (i32.const 10) (i32.const 128))
    (call $noticed_file)

    ;; 6. `x` at `no-such-branch`
    (call $read_file (local.get $repo) (local.get $repo_length)
      (i32.const 1) (i32.const 368) (i32.const 14)
      (i32.const 336) (i32.const 1) (i32.const 128))
    (call $noticed_file)

    ;; 7. `missing.md` at HEAD
    (call $read_file (local.get $repo) (local.get $repo_length)
      (i32.const 0) (i32.const 0) (i32.const 0)
      (i32.const 384) (i32.const 10) (i32.const 128))
    (call $noticed_file)

    (i32.store (i32.const 16) (i32.const 416))
    (i32.store (i32.const 20) (i32.const 4))
    (i32.const 16)))
