//! One module per component: its story cells and an optional demo.

pub mod avatar;
pub mod badge;
pub mod button;
pub mod card;
pub mod icon_button;
pub mod kbd;
pub mod separator;
pub mod skeleton;
pub mod spinner;
pub mod tokens;

#[cfg(test)]
mod tests {
    use crate::registry::{self, Pin};

    /// Every cell of every story builds on the GPUI side: a story naming a
    /// prop or value its component does not handle fails here, not as a
    /// blank or wrong cell on the Compare tab.
    #[test]
    fn every_story_cell_builds() {
        let mut failures = Vec::new();
        for entry in registry::entries() {
            for cell in entry.story.cells() {
                let pin = Pin {
                    state: cell.state,
                    freeze: None,
                };
                if let Err(error) = (entry.cell)(&cell.key, &cell.props, pin) {
                    failures.push(format!("{}: {}: {error}", entry.story.slug, cell.key));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    /// Story slugs are unique and match their file, which the generator and
    /// the reference directories are named by.
    #[test]
    fn slugs_are_unique() {
        let mut slugs: Vec<_> = registry::entries()
            .iter()
            .map(|e| e.story.slug.clone())
            .chain(registry::pages().iter().map(|p| p.slug.to_owned()))
            .collect();
        let count = slugs.len();
        slugs.sort();
        slugs.dedup();
        assert_eq!(slugs.len(), count, "duplicate slug");
    }
}
