
// #[derive(Clone)]
// pub struct Handle<T> {
//     pub pointer: Arc<T>,
// }

// impl<T> Handle<T> {
//     pub fn as_ref() -> &T {
//         pointer
//     }
// }

// impl <T: PartialEq> PartialEq for Handle<T> {
//     fn eq(&self, other: &Self) -> bool {
//         self.pointer.as_ref() == other.pointer.as_ref()
//     }
// }

// impl<T: PartialOrd> PartialOrd for Handle<T> {
//     fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
//         self.pointer.as_ref().partial_cmp(&other.unwrap())
//     }
// }

// impl <T: Eq> Eq for Handle<T> {

// }

// impl<T: Ord> Ord for Handle<T>
// {
//     fn cmp(&self, other: &Self) -> std::cmp::Ordering {
//         self.unwrap().cmp(&other.unwrap())
//     }
// }
