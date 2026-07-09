//! `rustysnes-script` — sandboxed Lua 5.4 scripting: memory read/write + a per-frame callback.
//!
//! Ported from RustyNES's proven `rustynes-script` shape (confirmed by reading its source
//! directly, not invented): a stripped-down Lua standard library (no `io`/`os`/`require` — a
//! script cannot touch the filesystem, network, or spawn a process), a runaway-loop guard (a
//! per-frame VM instruction budget, checked via `mlua`'s instruction-count hook), and
//! `mlua::Lua::scope` to bind `emu.read`/`emu.write` against a live `&mut Bus` for the duration
//! of exactly one [`ScriptEngine::on_frame`] call — the borrow never escapes into the Lua state,
//! so nothing here needs `Rc<RefCell<Bus>>` or similar permanent sharing.
//!
//! Native only (`mlua`'s vendored Lua VM needs a C compiler + `std`, neither available on
//! `wasm32`) — `rustysnes-frontend` depends on this crate only under
//! `target.'cfg(not(target_arch = "wasm32"))'.dependencies`.
//!
//! TAS movie record/playback lives in `rustysnes_core::movie` instead, deliberately NOT here —
//! it is pure input-log data + a capture/apply loop with no Lua/scripting coupling, and belongs
//! in the deterministic core alongside the save-state envelope it builds on
//! (`docs/adr/0006`), matching RustyNES's own crate boundary (confirmed by reading its source).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use mlua::{Function, HookTriggers, Lua, RegistryKey, StdLib, Table, VmState};
use rustysnes_core::Bus;

/// Per-frame Lua-VM instruction budget (checked every [`HOOK_INTERVAL`] instructions) — a
/// runaway `while true do end` script is interrupted rather than hanging the emulator. ~1M Lua
/// VM instructions is comfortably sub-frame-time on any host this targets.
const DEFAULT_INSTRUCTION_BUDGET: u64 = 1_000_000;
/// How often the interrupt hook checks the budget (every Nth VM instruction) — checking on every
/// single instruction would add real per-instruction overhead; this amortizes the check.
const HOOK_INTERVAL: u32 = 10_000;

/// Errors from loading or running a script.
#[derive(Debug, thiserror::Error)]
pub enum ScriptError {
    /// A Lua parse/runtime error, or the instruction-budget guard tripping.
    #[error("script error: {0}")]
    Lua(#[from] mlua::Error),
}

/// A sandboxed Lua 5.4 engine bound to no emulator state until [`Self::on_frame`] is called.
///
/// `emu.read(addr)` / `emu.write(addr, val)` reach [`Bus::peek_wram`]/[`Bus::poke_wram`] (WRAM
/// only, not the full 24-bit bus — a script wants stable memory-watch semantics, not to trip PPU/
/// APU/DMA register side effects by poking at arbitrary bus addresses). `emu.onFrame(fn)`
/// registers a callback invoked once per [`Self::on_frame`] call. `print(...)` is redirected into
/// an internal log ([`Self::drain_log`]) rather than stdout, so a hosting frontend can surface it
/// in its own UI.
pub struct ScriptEngine {
    lua: Lua,
    frame_cbs: Rc<RefCell<Vec<RegistryKey>>>,
    log: Rc<RefCell<Vec<String>>>,
    writes_locked: Rc<Cell<bool>>,
    instruction_count: Rc<Cell<u64>>,
    instruction_budget: Rc<Cell<u64>>,
}

impl ScriptEngine {
    /// Build a fresh, sandboxed engine with no script loaded yet.
    ///
    /// # Errors
    /// Returns [`ScriptError`] if the Lua VM or its sandbox prelude fails to construct (an
    /// `mlua` internal error — not expected in normal operation).
    pub fn new() -> Result<Self, ScriptError> {
        let lua = Lua::new_with(
            StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::COROUTINE,
            mlua::LuaOptions::default(),
        )?;
        // Belt-and-suspenders against a future `StdLib` bit-flag change accidentally widening the
        // sandbox: explicitly nil out everything that could reach the filesystem, network, a
        // subprocess, or arbitrary bytecode loading, even though the chosen `StdLib` bits above
        // already exclude `io`/`os`.
        for name in [
            "load",
            "loadfile",
            "dofile",
            "loadstring",
            "collectgarbage",
            "require",
            "package",
            "io",
            "os",
            "debug",
        ] {
            lua.globals().set(name, mlua::Value::Nil)?;
        }

        let instruction_count = Rc::new(Cell::new(0u64));
        let instruction_budget = Rc::new(Cell::new(DEFAULT_INSTRUCTION_BUDGET));
        {
            let count = Rc::clone(&instruction_count);
            let budget = Rc::clone(&instruction_budget);
            lua.set_hook(
                HookTriggers::new().every_nth_instruction(HOOK_INTERVAL),
                move |_lua, _debug| {
                    let n = count.get() + u64::from(HOOK_INTERVAL);
                    count.set(n);
                    if n > budget.get() {
                        Err(mlua::Error::RuntimeError(
                            "script exceeded its per-frame instruction budget".into(),
                        ))
                    } else {
                        Ok(VmState::Continue)
                    }
                },
            )?;
        }

        let log: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        {
            let log = Rc::clone(&log);
            // Standard Lua `print` accepts any number of arguments of any type, formatting each
            // via `tostring` and joining with tabs — a single-`String`-argument version raises a
            // type error on `print(123)` or `print("a", "b")`, which is surprising enough to
            // unload a script that would otherwise run fine.
            let print_fn = lua.create_function(move |lua, args: mlua::MultiValue| {
                let tostring: Function = lua.globals().get("tostring")?;
                let mut parts = Vec::with_capacity(args.len());
                for arg in args {
                    parts.push(tostring.call::<String>(arg)?);
                }
                log.borrow_mut().push(parts.join("\t"));
                Ok(())
            })?;
            lua.globals().set("print", print_fn)?;
        }

        let frame_cbs: Rc<RefCell<Vec<RegistryKey>>> = Rc::new(RefCell::new(Vec::new()));
        let emu = lua.create_table()?;
        {
            let cbs = Rc::clone(&frame_cbs);
            let on_frame_fn = lua.create_function(move |lua, f: Function| {
                cbs.borrow_mut().push(lua.create_registry_value(f)?);
                Ok(())
            })?;
            emu.set("onFrame", on_frame_fn)?;
        }
        lua.globals().set("emu", emu)?;

        Ok(Self {
            lua,
            frame_cbs,
            log,
            writes_locked: Rc::new(Cell::new(false)),
            instruction_count,
            instruction_budget,
        })
    }

    /// Parse and execute `src` (a script's top level — typically just `emu.onFrame(...)`
    /// registration; the per-frame body runs later via [`Self::on_frame`]).
    ///
    /// Replaces any script previously loaded into this engine: registered `emu.onFrame`
    /// callbacks are cleared first, so loading a second script doesn't stack its callbacks on
    /// top of the first's.
    ///
    /// # Errors
    /// Returns [`ScriptError`] on a Lua parse error, a runtime error during the top-level
    /// execution, or the instruction budget tripping.
    pub fn load(&mut self, src: &str) -> Result<(), ScriptError> {
        for key in self.frame_cbs.borrow_mut().drain(..) {
            let _ = self.lua.remove_registry_value(key);
        }
        self.instruction_count.set(0);
        self.lua.load(src).exec()?;
        Ok(())
    }

    /// Gate `emu.write` — when `true`, writes are silently dropped (a no-op, not an error) so a
    /// script can never perturb a deterministic run it doesn't own (TAS movie record/playback,
    /// a future netplay session). Reads are never gated; observing state is always safe.
    pub fn set_writes_locked(&self, locked: bool) {
        self.writes_locked.set(locked);
    }

    /// Override the default per-frame instruction budget (1,000,000).
    pub fn set_instruction_budget(&self, budget: u64) {
        self.instruction_budget.set(budget);
    }

    /// Drain and return every `print(...)` call since the last drain.
    #[must_use]
    pub fn drain_log(&self) -> Vec<String> {
        core::mem::take(&mut self.log.borrow_mut())
    }

    /// Run one frame: reset the instruction counter, bind `emu.read`/`emu.write` against `bus`
    /// for the duration of this call only, then invoke every registered `onFrame` callback in
    /// registration order.
    ///
    /// # Errors
    /// Returns [`ScriptError`] if a callback raises a Lua error or the instruction budget trips
    /// mid-frame (the callback is aborted; earlier callbacks that already ran this frame are not
    /// rolled back — this mirrors a script crashing partway through a single frame's logic, not
    /// a transactional unit).
    pub fn on_frame(&mut self, bus: &mut Bus) -> Result<(), ScriptError> {
        self.instruction_count.set(0);
        let bus_cell = RefCell::new(bus);
        let writes_locked = self.writes_locked.get();
        let lua = &self.lua;
        let frame_cbs = &self.frame_cbs;

        lua.scope(|scope| {
            let emu: Table = lua.globals().get("emu")?;

            let read =
                scope.create_function(|_, addr: u32| Ok(bus_cell.borrow_mut().peek_wram(addr)))?;
            emu.set("read", read)?;

            let write = scope.create_function(|_, (addr, val): (u32, u8)| {
                if !writes_locked {
                    bus_cell.borrow_mut().poke_wram(addr, val);
                }
                Ok(())
            })?;
            emu.set("write", write)?;

            let callbacks: Vec<Function> = frame_cbs
                .borrow()
                .iter()
                .map(|key| lua.registry_value(key))
                .collect::<mlua::Result<_>>()?;
            for f in callbacks {
                f.call::<()>(())?;
            }
            Ok(())
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use rustysnes_core::System;

    use super::*;

    #[test]
    fn on_frame_invokes_registered_callback() {
        let mut engine = ScriptEngine::new().expect("engine builds");
        engine
            .load("emu.onFrame(function() print('hello from lua') end)")
            .expect("script loads");
        let mut sys = System::new(0);
        engine.on_frame(&mut sys.bus).expect("frame runs");
        assert_eq!(engine.drain_log(), vec!["hello from lua"]);
    }

    #[test]
    fn read_write_round_trips_through_wram() {
        let mut engine = ScriptEngine::new().expect("engine builds");
        engine
            .load("emu.onFrame(function() emu.write(0x7E0010, emu.read(0x7E0010) + 1) end)")
            .expect("script loads");
        let mut sys = System::new(0);
        sys.bus.poke_wram(0x7E_0010, 41);
        engine.on_frame(&mut sys.bus).expect("frame runs");
        assert_eq!(sys.bus.peek_wram(0x7E_0010), 42);
    }

    #[test]
    fn writes_locked_makes_write_a_silent_no_op() {
        let mut engine = ScriptEngine::new().expect("engine builds");
        engine.set_writes_locked(true);
        engine
            .load("emu.onFrame(function() emu.write(0x7E0010, 99) end)")
            .expect("script loads");
        let mut sys = System::new(0);
        sys.bus.poke_wram(0x7E_0010, 7);
        engine
            .on_frame(&mut sys.bus)
            .expect("frame runs (write is just a no-op, not an error)");
        assert_eq!(sys.bus.peek_wram(0x7E_0010), 7);
    }

    #[test]
    fn runaway_loop_is_interrupted_by_the_instruction_budget() {
        let mut engine = ScriptEngine::new().expect("engine builds");
        engine.set_instruction_budget(50_000);
        engine
            .load("emu.onFrame(function() while true do end end)")
            .expect("script loads");
        let mut sys = System::new(0);
        let result = engine.on_frame(&mut sys.bus);
        assert!(
            result.is_err(),
            "runaway loop must be interrupted, not hang"
        );
    }

    #[test]
    fn print_accepts_multiple_and_non_string_arguments() {
        let mut engine = ScriptEngine::new().expect("engine builds");
        engine
            .load("emu.onFrame(function() print('Score:', 123, nil, true) end)")
            .expect("script loads");
        let mut sys = System::new(0);
        engine.on_frame(&mut sys.bus).expect("frame runs");
        assert_eq!(engine.drain_log(), vec!["Score:\t123\tnil\ttrue"]);
    }

    #[test]
    fn load_replaces_previously_registered_frame_callbacks() {
        let mut engine = ScriptEngine::new().expect("engine builds");
        engine
            .load("emu.onFrame(function() print('first') end)")
            .expect("first script loads");
        engine
            .load("emu.onFrame(function() print('second') end)")
            .expect("second script loads");
        let mut sys = System::new(0);
        engine.on_frame(&mut sys.bus).expect("frame runs");
        assert_eq!(
            engine.drain_log(),
            vec!["second"],
            "loading a new script must clear callbacks from any previously loaded script"
        );
    }

    #[test]
    fn sandbox_has_no_filesystem_or_process_access() {
        let mut engine = ScriptEngine::new().expect("engine builds");
        for src in [
            "emu.onFrame(function() io.open('/etc/passwd') end)",
            "emu.onFrame(function() os.execute('true') end)",
            "emu.onFrame(function() require('os') end)",
        ] {
            engine
                .load(src)
                .expect("script loads (the error is at call time, not parse time)");
            let mut sys = System::new(0);
            assert!(
                engine.on_frame(&mut sys.bus).is_err(),
                "sandboxed script must not be able to call {src}"
            );
        }
    }
}
