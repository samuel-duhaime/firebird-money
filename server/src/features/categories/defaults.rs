//! The `DefaultCategory` shape used by `category_groups::defaults` to describe the starter
//! categories nested inside each default group — living here since categories are what this shape
//! ultimately becomes rows of.

pub struct DefaultCategory {
    pub name_en: &'static str,
    pub name_fr: &'static str,
}
