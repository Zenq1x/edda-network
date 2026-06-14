;; Counter smart contract for Edda Network
;; Stores a u64 counter in account[0].data, increments on each call.
;;
;; Build:  wat2wasm counter.wat -o counter.wasm
;; Deploy: edda-node RPC deployProgram (base64-encode counter.wasm)
;;
;; Accounts expected:
;;   [0] writable — counter state account (8 bytes, initialized to 0)

(module
  ;; ── Edda host functions ────────────────────────────────────────────────────
  (import "env" "edda_log"      (func $log      (param i32 i32)))
  (import "env" "edda_data_len" (func $data_len (param i32) (result i32)))
  (import "env" "edda_get_data" (func $get_data (param i32 i32 i32) (result i32)))
  (import "env" "edda_set_data" (func $set_data (param i32 i32 i32)))

  (memory (export "memory") 1)

  ;; Static strings
  (data (i32.const 100) "Counter incremented to: ")
  (data (i32.const 200) "0123456789")

  ;; Scratch space: [300..308) = u64 counter, [400..432) = log string buffer

  ;; ── u64 little-endian helpers ─────────────────────────────────────────────
  (func $load_u64 (result i64)
    (i64.load (i32.const 300))
  )
  (func $store_u64 (param $v i64)
    (i64.store (i32.const 300) (local.get $v))
  )

  ;; ── Format u64 as decimal into [400..] ────────────────────────────────────
  ;; Returns length of the written string.
  (func $u64_to_dec (param $n i64) (result i32)
    (local $pos i32)
    (local $start i32)
    (local $end i32)
    (local $tmp i32)
    (local $c i32)
    (local $half i32)

    (local.set $pos   (i32.const 400))
    (local.set $start (i32.const 400))

    ;; Special case: zero
    (if (i64.eqz (local.get $n))
      (then
        (i32.store8 (i32.const 400) (i32.const 48))  ;; '0'
        (return (i32.const 1))
      )
    )

    ;; Write digits in reverse
    (block $break
      (loop $loop
        (br_if $break (i64.eqz (local.get $n)))
        (local.set $c
          (i32.add
            (i32.const 48)
            (i32.wrap_i64 (i64.rem_u (local.get $n) (i64.const 10)))
          )
        )
        (i32.store8 (local.get $pos) (local.get $c))
        (local.set $n   (i64.div_u (local.get $n) (i64.const 10)))
        (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
        (br $loop)
      )
    )

    (local.set $end (i32.sub (local.get $pos) (i32.const 1)))

    ;; Reverse the string in place
    (block $rev_break
      (loop $rev
        (br_if $rev_break (i32.ge_u (local.get $start) (local.get $end)))
        (local.set $tmp (i32.load8_u (local.get $start)))
        (i32.store8 (local.get $start) (i32.load8_u (local.get $end)))
        (i32.store8 (local.get $end)   (local.get $tmp))
        (local.set $start (i32.add (local.get $start) (i32.const 1)))
        (local.set $end   (i32.sub (local.get $end)   (i32.const 1)))
        (br $rev)
      )
    )

    (i32.sub (local.get $pos) (i32.const 400))
  )

  ;; ── Entry point ────────────────────────────────────────────────────────────
  (func $process_instruction (export "process_instruction") (param $data_ptr i32) (param $data_len i32)
    (local $count i64)
    (local $digits i32)
    (local $msg_len i32)

    ;; Load current counter from account[0] data (8 bytes LE)
    (drop (call $get_data (i32.const 0) (i32.const 300) (i32.const 8)))

    ;; Increment
    (local.set $count (i64.add (call $load_u64) (i64.const 1)))
    (call $store_u64 (local.get $count))

    ;; Write back to account[0]
    (call $set_data (i32.const 0) (i32.const 300) (i32.const 8))

    ;; Build log message "Counter incremented to: <N>"
    ;; Copy prefix (24 chars) into [450..]
    (memory.copy
      (i32.const 450)
      (i32.const 100)
      (i32.const 24)
    )

    ;; Format counter value into [400..]
    (local.set $digits (call $u64_to_dec (local.get $count)))

    ;; Copy digits after prefix
    (memory.copy
      (i32.add (i32.const 450) (i32.const 24))
      (i32.const 400)
      (local.get $digits)
    )

    (local.set $msg_len (i32.add (i32.const 24) (local.get $digits)))

    (call $log (i32.const 450) (local.get $msg_len))
  )
)
