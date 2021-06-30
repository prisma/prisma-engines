pub trait VecUtilities<T: Clone> {
    fn clone_push(&self, item: &T) -> Self;
    fn clone_append(&self, other: &mut Vec<T>) -> Self;
}

impl<T: Clone> VecUtilities<T> for Vec<T> {
    fn clone_push(&self, item: &T) -> Self {
        let mut new_vec = self.clone();
        new_vec.push(item.clone());
        new_vec
    }

    fn clone_append(&self, other: &mut Vec<T>) -> Self {
        let mut new_vec = self.clone();
        new_vec.append(other);

        new_vec
    }
}
