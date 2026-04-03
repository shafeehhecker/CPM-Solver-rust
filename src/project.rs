// project.rs — Project state management
// Handles: project metadata, activity list, undo/redo, save/load (JSON)

use serde::{Deserialize, Serialize};
use chrono::NaiveDate;
use std::path::PathBuf;
use crate::activity::{Activity, Predecessor, RelType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub name: String,
    pub start_date: Option<NaiveDate>,
    pub description: String,
    pub version: u32,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        ProjectSettings {
            name: "Untitled Project".to_string(),
            start_date: None,
            description: String::new(),
            version: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub settings: ProjectSettings,
    pub activities: Vec<Activity>,
    #[serde(skip)]
    pub is_dirty: bool,
    #[serde(skip)]
    pub file_path: Option<PathBuf>,
    #[serde(skip)]
    pub is_scheduled: bool,
}

impl Default for Project {
    fn default() -> Self {
        Project {
            settings: ProjectSettings::default(),
            activities: Vec::new(),
            is_dirty: false,
            file_path: None,
            is_scheduled: false,
        }
    }
}

impl Project {
    pub fn load_sample() -> Self {
        use crate::activity::Activity;

        let activities = vec![
            Activity::new("A", "Start", 2.0),
            Activity::new("B", "Foundation", 4.0).with_predecessor("A"),
            Activity {
                id: "C".to_string(),
                name: "Structure".to_string(),
                duration: 6.0,
                predecessors: vec![Predecessor { activity_id: "B".to_string(), rel_type: RelType::FS, lag: 0 }],
                resource: String::new(),
                wbs: String::new(),
                cpm: Default::default(),
            },
            Activity {
                id: "D".to_string(),
                name: "Electrical".to_string(),
                duration: 3.0,
                predecessors: vec![Predecessor { activity_id: "B".to_string(), rel_type: RelType::FS, lag: 0 }],
                resource: "Electrical Team".to_string(),
                wbs: String::new(),
                cpm: Default::default(),
            },
            Activity {
                id: "E".to_string(),
                name: "Finish".to_string(),
                duration: 2.0,
                predecessors: vec![
                    Predecessor { activity_id: "C".to_string(), rel_type: RelType::FS, lag: 0 },
                    Predecessor { activity_id: "D".to_string(), rel_type: RelType::FS, lag: 0 },
                ],
                resource: String::new(),
                wbs: String::new(),
                cpm: Default::default(),
            },
        ];

        Project {
            settings: ProjectSettings {
                name: "Sample Construction Project".to_string(),
                start_date: Some(NaiveDate::from_ymd_opt(2025, 1, 6).unwrap()),
                description: "Demo project for CPM scheduler".to_string(),
                version: 1,
            },
            activities,
            is_dirty: false,
            file_path: None,
            is_scheduled: false,
        }
    }

    /// Save project to JSON file
    pub fn save_to_file(&self, path: &PathBuf) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Serialization error: {}", e))?;
        std::fs::write(path, json)
            .map_err(|e| format!("Write error: {}", e))?;
        Ok(())
    }

    /// Load project from JSON file
    pub fn load_from_file(path: &PathBuf) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Read error: {}", e))?;
        let mut project: Project = serde_json::from_str(&content)
            .map_err(|e| format!("Parse error: {}", e))?;
        project.file_path = Some(path.clone());
        project.is_dirty = false;
        Ok(project)
    }

    pub fn add_activity(&mut self, act: Activity) {
        self.activities.push(act);
        self.is_dirty = true;
        self.is_scheduled = false;
    }

    pub fn remove_activity(&mut self, id: &str) {
        // Remove any predecessor references to this ID
        for act in &mut self.activities {
            act.predecessors.retain(|p| p.activity_id != id);
        }
        self.activities.retain(|a| a.id != id);
        self.is_dirty = true;
        self.is_scheduled = false;
    }

    pub fn update_activity(&mut self, updated: Activity) {
        if let Some(act) = self.activities.iter_mut().find(|a| a.id == updated.id) {
            // Preserve CPM results if nothing structural changed
            let old_cpm = act.cpm.clone();
            *act = updated;
            // If duration/predecessors changed, CPM is stale
            self.is_dirty = true;
            self.is_scheduled = false;
        }
    }

    pub fn title(&self) -> String {
        let dirty = if self.is_dirty { " •" } else { "" };
        let file = self.file_path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|s| format!(" — {}", s))
            .unwrap_or_default();
        format!("CPM Scheduler — {}{}{}", self.settings.name, file, dirty)
    }
}
