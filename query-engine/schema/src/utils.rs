pub fn capitalize<T>(s: T) -> String
where
    T: Into<String>,
{
    let s = s.into();

    // This is safe to unwrap, as the validation regex for model / field
    // names used in the data model essentially guarantees ASCII.
    let first_char = s.chars().next().unwrap();

    format!("{}{}", first_char.to_uppercase(), s[1..].to_owned())
}
