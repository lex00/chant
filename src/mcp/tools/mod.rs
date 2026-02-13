//! MCP tools implementations

pub mod lifecycle;
pub mod spec;
pub mod watch;
pub mod work;

pub use lifecycle::{tool_chant_archive, tool_chant_cancel, tool_chant_finalize, tool_chant_reset};
pub use spec::{
    tool_chant_add, tool_chant_diagnose, tool_chant_lint, tool_chant_log, tool_chant_ready,
    tool_chant_search, tool_chant_spec_get, tool_chant_spec_list, tool_chant_spec_update,
    tool_chant_status, tool_chant_verify,
};
pub use watch::{tool_chant_watch_start, tool_chant_watch_status, tool_chant_watch_stop};
pub use work::{
    tool_chant_pause, tool_chant_split, tool_chant_takeover, tool_chant_work_list,
    tool_chant_work_start,
};
