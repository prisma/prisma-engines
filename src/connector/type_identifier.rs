pub(crate) trait TypeIdentifier {
    fn is_real(&self) -> bool;
    fn is_float(&self) -> bool;
    fn is_double(&self) -> bool;
    fn is_int32(&self) -> bool;
    fn is_int64(&self) -> bool;
    fn is_datetime(&self) -> bool;
    fn is_time(&self) -> bool;
    fn is_date(&self) -> bool;
    fn is_text(&self) -> bool;
    fn is_bytes(&self) -> bool;
    fn is_bool(&self) -> bool;
    fn is_json(&self) -> bool;
    fn is_enum(&self) -> bool;
    fn is_null(&self) -> bool;
}
