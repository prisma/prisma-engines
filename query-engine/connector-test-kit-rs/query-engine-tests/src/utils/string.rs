pub fn enclose(input: &str, with: &str) -> String {
    format!("{}{}{}", with, input, with)
}

pub fn enclose_all<T>(input: Vec<T>, with: &str) -> Vec<String>
where
    T: AsRef<str>,
{
    input.into_iter().map(|el| enclose(el.as_ref(), with)).collect()
}

pub fn stringify<T>(input: Vec<T>) -> Vec<String>
where
    T: ToString,
{
    input.iter().map(ToString::to_string).collect()
}
