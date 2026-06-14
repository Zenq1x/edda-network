//! WASM smart contract runtime for Edda Network.
//! Each program exports `process_instruction(data_ptr: i32, data_len: i32)` and `memory`.

use anyhow::{anyhow, Result};
use wasmtime::{Config, Engine, Linker, Module, Store};

/// Maximum fuel (computation units) per contract invocation — ~1M instructions.
pub const MAX_GAS: u64 = 1_000_000;

// ── Shared state passed into the WASM store ───────────────────────────────────

#[derive(Debug, Clone)]
pub struct AccountView {
    pub lamports: u64,
    pub data:     Vec<u8>,
    pub writable: bool,
}

pub struct ProgramContext {
    pub accounts: Vec<AccountView>,
    pub logs:     Vec<String>,
    pub input:    Vec<u8>,
}

#[derive(Debug)]
pub struct ExecutionResult {
    pub accounts: Vec<AccountView>,
    pub logs:     Vec<String>,
    pub gas_used: u64,
}

// ── Runtime ───────────────────────────────────────────────────────────────────

pub struct WasmRuntime {
    engine: Engine,
}

impl WasmRuntime {
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.consume_fuel(true); // enable gas metering
        let engine = Engine::new(&config)?;
        Ok(Self { engine })
    }

    pub fn compile(&self, bytecode: &[u8]) -> Result<Module> {
        Module::new(&self.engine, bytecode).map_err(|e| anyhow!("compile: {}", e))
    }

    pub fn execute(
        &self,
        module:   &Module,
        accounts: Vec<AccountView>,
        input:    Vec<u8>,
    ) -> Result<ExecutionResult> {
        let mut store = Store::new(
            &self.engine,
            ProgramContext { accounts, logs: Vec::new(), input: input.clone() },
        );
        store.set_fuel(MAX_GAS)?;

        let mut linker: Linker<ProgramContext> = Linker::new(&self.engine);

        // edda_log(ptr, len)
        linker.func_wrap("env", "edda_log",
            |mut caller: wasmtime::Caller<'_, ProgramContext>, ptr: i32, len: i32| {
                let mem = match caller.get_export("memory") {
                    Some(wasmtime::Extern::Memory(m)) => m, _ => return,
                };
                let bytes: Vec<u8> = {
                    let data = mem.data(&caller);
                    let s = ptr as usize;
                    let e = (s + len as usize).min(data.len());
                    data[s..e].to_vec()
                };
                if let Ok(s) = std::str::from_utf8(&bytes) {
                    caller.data_mut().logs.push(s.to_string());
                }
            },
        )?;

        // edda_lamports(idx) -> i64
        linker.func_wrap("env", "edda_lamports",
            |caller: wasmtime::Caller<'_, ProgramContext>, idx: i32| -> i64 {
                caller.data().accounts.get(idx as usize)
                    .map(|a| a.lamports as i64).unwrap_or(0)
            },
        )?;

        // edda_transfer(from, to, amount)
        linker.func_wrap("env", "edda_transfer",
            |mut caller: wasmtime::Caller<'_, ProgramContext>, from: i32, to: i32, amount: i64| {
                let ctx = caller.data_mut();
                let f = from as usize; let t = to as usize; let amt = amount as u64;
                if f < ctx.accounts.len() && t < ctx.accounts.len()
                    && ctx.accounts[f].writable && ctx.accounts[f].lamports >= amt
                {
                    ctx.accounts[f].lamports -= amt;
                    ctx.accounts[t].lamports += amt;
                }
            },
        )?;

        // edda_data_len(idx) -> i32
        linker.func_wrap("env", "edda_data_len",
            |caller: wasmtime::Caller<'_, ProgramContext>, idx: i32| -> i32 {
                caller.data().accounts.get(idx as usize)
                    .map(|a| a.data.len() as i32).unwrap_or(0)
            },
        )?;

        // edda_get_data(idx, dst_ptr, max_len) -> i32 bytes copied
        linker.func_wrap("env", "edda_get_data",
            |mut caller: wasmtime::Caller<'_, ProgramContext>, idx: i32, dst: i32, max: i32| -> i32 {
                let src: Vec<u8> = caller.data().accounts.get(idx as usize)
                    .map(|a| a.data.clone()).unwrap_or_default();
                let n = src.len().min(max as usize);
                if n == 0 { return 0; }
                let mem = match caller.get_export("memory") {
                    Some(wasmtime::Extern::Memory(m)) => m, _ => return 0,
                };
                mem.write(&mut caller, dst as usize, &src[..n]).ok();
                n as i32
            },
        )?;

        // edda_set_data(idx, src_ptr, len)
        linker.func_wrap("env", "edda_set_data",
            |mut caller: wasmtime::Caller<'_, ProgramContext>, idx: i32, src: i32, len: i32| {
                let mem = match caller.get_export("memory") {
                    Some(wasmtime::Extern::Memory(m)) => m, _ => return,
                };
                let bytes: Vec<u8> = {
                    let data = mem.data(&caller);
                    let s = src as usize;
                    let e = (s + len as usize).min(data.len());
                    data[s..e].to_vec()
                };
                let ctx = caller.data_mut();
                if let Some(acc) = ctx.accounts.get_mut(idx as usize) {
                    if acc.writable { acc.data = bytes; }
                }
            },
        )?;

        let instance = linker.instantiate(&mut store, module)?;

        // Copy instruction data into WASM memory at offset 0
        if let Some(wasmtime::Extern::Memory(mem)) = instance.get_export(&mut store, "memory") {
            let data_copy = store.data().input.clone();
            if !data_copy.is_empty() {
                mem.write(&mut store, 0, &data_copy)
                    .map_err(|e| anyhow!("memory write: {}", e))?;
            }
        }

        let func = instance
            .get_typed_func::<(i32, i32), ()>(&mut store, "process_instruction")
            .map_err(|_| anyhow!("missing export: process_instruction(i32, i32)"))?;

        let input_len = store.data().input.len() as i32;
        func.call(&mut store, (0, input_len))
            .map_err(|e| {
                // Friendly message for out-of-gas
                if e.to_string().contains("fuel") {
                    anyhow!("out of gas (limit: {} units)", MAX_GAS)
                } else {
                    anyhow!("execution trap: {}", e)
                }
            })?;

        let gas_used = MAX_GAS - store.get_fuel().unwrap_or(MAX_GAS);
        let ctx = store.into_data();
        Ok(ExecutionResult { accounts: ctx.accounts, logs: ctx.logs, gas_used })
    }
}

impl Default for WasmRuntime {
    fn default() -> Self { Self::new().expect("wasmtime engine init failed") }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execute_hello_contract() {
        let rt = WasmRuntime::new().unwrap();
        let wasm = wat::parse_str(r#"
            (module
              (import "env" "edda_log" (func $log (param i32 i32)))
              (memory (export "memory") 1)
              (data (i32.const 64) "hello edda")
              (func (export "process_instruction") (param i32 i32)
                (call $log (i32.const 64) (i32.const 10))
              )
            )
        "#).unwrap();
        let module = rt.compile(&wasm).unwrap();
        let result = rt.execute(&module, vec![], vec![]).unwrap();
        assert_eq!(result.logs, vec!["hello edda"]);
        assert!(result.gas_used > 0, "gas should be tracked");
    }

    #[test]
    fn transfer_between_accounts() {
        let rt = WasmRuntime::new().unwrap();
        let wasm = wat::parse_str(r#"
            (module
              (import "env" "edda_log"      (func $log  (param i32 i32)))
              (import "env" "edda_transfer" (func $xfer (param i32 i32 i64)))
              (memory (export "memory") 1)
              (func (export "process_instruction") (param i32 i32)
                (call $xfer (i32.const 0) (i32.const 1) (i64.const 100))
              )
            )
        "#).unwrap();
        let accounts = vec![
            AccountView { lamports: 500, data: vec![], writable: true },
            AccountView { lamports: 0,   data: vec![], writable: true },
        ];
        let module = rt.compile(&wasm).unwrap();
        let result = rt.execute(&module, accounts, vec![]).unwrap();
        assert_eq!(result.accounts[0].lamports, 400);
        assert_eq!(result.accounts[1].lamports, 100);
    }

    #[test]
    fn gas_limit_enforced() {
        let rt = WasmRuntime::new().unwrap();
        // Infinite loop — must be killed by gas limiter
        let wasm = wat::parse_str(r#"
            (module
              (memory (export "memory") 1)
              (func (export "process_instruction") (param i32 i32)
                (loop $l (br $l))
              )
            )
        "#).unwrap();
        let module = rt.compile(&wasm).unwrap();
        let result = rt.execute(&module, vec![], vec![]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("gas"));
    }
}
