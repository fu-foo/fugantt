//! The arithmetic of a Gantt chart, and nothing that stores one.
//!
//! Hand [`domain::build`] the rows as they are kept and the settings around
//! them, and it answers with what the grid draws: the outline flattened, each
//! summary row added up from what is under it, the days counted the way the
//! project counts them, and what is late said plainly.
//!
//! Where the rows are kept, who may read them and how they arrive over the
//! wire are the host's business.

pub mod domain;
pub mod sortkey;
pub mod text;
pub mod wire;
