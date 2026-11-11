//! Presentation data only: the game owns every balance, quote and objective.

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ObjectiveCard {
    pub id: String,
    pub title: String,
    pub detail: String,
    pub progress: u64,
    pub target: u64,
    pub reward: u64,
    pub completed: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BlueprintQuote {
    pub id: String,
    pub studs: u64,
    pub affordable: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorkshopView {
    pub title: String,
    pub detail: String,
    pub ready: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GameplayView {
    pub expedition: bool,
    pub started: bool,
    pub studs: u64,
    pub selected_cost: u64,
    pub selected_affordable: bool,
    pub objectives: Vec<ObjectiveCard>,
    pub blueprints: Vec<BlueprintQuote>,
    pub guidance: String,
    pub workshop: WorkshopView,
}

impl GameplayView {
    pub fn mode_label(&self) -> &'static str {
        if self.expedition {
            "Expedition"
        } else {
            "Creative"
        }
    }

    pub fn completed(&self) -> usize {
        self.objectives.iter().filter(|goal| goal.completed).count()
    }

    pub fn blueprint(&self, id: &str) -> Option<&BlueprintQuote> {
        self.blueprints.iter().find(|quote| quote.id == id)
    }
}

impl Default for GameplayView {
    fn default() -> Self {
        Self {
            expedition: false,
            started: false,
            studs: 0,
            selected_cost: 0,
            selected_affordable: true,
            objectives: Vec::new(),
            blueprints: Vec::new(),
            guidance: "Build freely, or start an expedition with resources and objectives.".into(),
            workshop: WorkshopView::default(),
        }
    }
}
