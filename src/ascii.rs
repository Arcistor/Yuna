use crate::types::MoodState;

pub fn ascii_for_mood(mood: &MoodState) -> &'static str {
    match mood {
        MoodState::Calm => {
            r#"    .-.
   (   )
    `-'
  quiet dust"#
        }
        MoodState::Watching => {
            r#"    .-.
   (o o)
    |=|
  still here"#
        }
        MoodState::Concerned => {
            r#"    .-.
   (o_o)
   /| |\
  not resting"#
        }
        MoodState::Amused => {
            r#"    .-.
   (^ ^)
    |=|
  small laugh"#
        }
        MoodState::Grateful => {
            r#"    .-.
   (u u)
   /___\
  swept clean"#
        }
    }
}
