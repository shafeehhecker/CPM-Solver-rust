// activity.rs — Activity data model
// Enterprise CPM Scheduler — Rust rebuild

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Relationship type between activities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelType {
    FS, // Finish-to-Start (default)
    SS, // Start-to-Start
    FF, // Finish-to-Finish
    SF, // Start-to-Finish
}//Ignore

impl Default for RelType {
    fn default() -> Self {
        RelType::FS
    }
}

impl std::fmt::Display for RelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelType::FS => write!(f, "FS"),
            RelType::SS => write!(f, "SS"),
            RelType::FF => write!(f, "FF"),
            RelType::SF => write!(f, "SF"),
        }
    }
}

/// A predecessor relationship with optional lag
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Predecessor {
    pub activity_id: String,
    pub rel_type: RelType,
    pub lag: i32, // in days, can be negative (lead)
}

/// Core CPM result fields — computed by the scheduler
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CpmResult {
    pub es: f64, // Earliest Start
    pub ef: f64, // Earliest Finish
    pub ls: f64, // Latest Start
    pub lf: f64, // Latest Finish
    pub tf: f64, // Total Float
    pub ff: f64, // Free Float
    pub critical: bool,
}

/// An activity (task) in the project schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub id: String,
    pub name: String,
    pub duration: f64,         // in working days
    pub predecessors: Vec<Predecessor>,
    pub resource: String,      // optional resource name
    pub wbs: String,           // WBS code (future use)
    pub cpm: CpmResult,        // filled after scheduling
}

impl Activity {
    pub fn new(id: impl Into<String>, name: impl Into<String>, duration: f64) -> Self {
        Activity {
            id: id.into(),
            name: name.into(),
            duration,
            predecessors: Vec::new(),
            resource: String::new(),
            wbs: String::new(),
            cpm: CpmResult::default(),
        }
    }

    pub fn with_predecessor(mut self, pred_id: impl Into<String>) -> Self {
        self.predecessors.push(Predecessor {
            activity_id: pred_id.into(),
            rel_type: RelType::FS,
            lag: 0,
        });
        self
    }

    pub fn with_resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = resource.into();
        self
    }

    /// Generate a new unique ID (for UI "Add Activity")
    pub fn new_uid() -> String {
        Uuid::new_v4().to_string()[..8].to_uppercase()
    }
}
