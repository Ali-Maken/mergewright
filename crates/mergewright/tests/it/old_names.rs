//! Every earlier generation's spellings, written out once for the guard
//! tests that check nothing still says them (pre-publish-cleanup/11b2; the
//! same list `rename_names::OLD_NAMES` holds flat).

/// One earlier generation's names.
pub struct Earlier {
    pub dir: &'static str,
    pub title: &'static str,
    pub command: &'static str,
    pub env_prefix: &'static str,
}

/// Oldest first.
pub const EARLIER: [Earlier; 2] = [
    Earlier {
        dir: "graft",
        title: "Graft",
        command: "graft",
        // Split so `rename_names::scripts::no_old_env_name` does not read it.
        env_prefix: concat!("GRAFT", "_"),
    },
    Earlier {
        dir: "tributary",
        title: "Tributary",
        command: "trib",
        env_prefix: concat!("TRIB", "_"),
    },
];
