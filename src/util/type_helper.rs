use std::{collections::HashMap, sync::Arc};

use fapi_diff::format::prototype::{
    ComplexType, Property, Prototype, PrototypeDoc, Type, TypeConcept,
};

struct DocHelper {
    docs: PrototypeDoc,
    base_link: Arc<str>,

    type2proto: HashMap<Arc<str>, u16>,
    name2proto: HashMap<Arc<str>, u16>,
    name2type: HashMap<Arc<str>, u16>,
}

impl DocHelper {
    pub fn new(docs: PrototypeDoc) -> Self {
        let mut type2proto = HashMap::new();
        let mut name2proto = HashMap::new();
        let mut name2type = HashMap::new();

        for (idx, proto) in docs.prototypes.iter().enumerate() {
            let idx = idx as u16;
            if !proto.typename.is_empty() {
                type2proto.insert(proto.typename.clone().into(), idx);
            }

            name2proto.insert(proto.name.clone().into(), idx);
        }

        for (idx, type_) in docs.types.iter().enumerate() {
            let idx = idx as u16;
            name2type.insert(type_.name.clone().into(), idx);
        }

        let base_link = format!("https://lua-api.factorio.com/{}", docs.application_version).into();

        Self {
            docs,
            base_link,
            type2proto,
            name2proto,
            name2type,
        }
    }

    pub fn get_proto_by_type(&self, type_: &str) -> Option<&Prototype> {
        let idx = self.type2proto.get(type_)?;
        self.docs.prototypes.get(*idx as usize)
    }

    pub fn get_proto(&self, name: &str) -> Option<&Prototype> {
        let idx = self.name2proto.get(name)?;
        self.docs.prototypes.get(*idx as usize)
    }

    pub fn get_type(&self, name: &str) -> Option<&TypeConcept> {
        let idx = self.name2type.get(name)?;
        self.docs.types.get(*idx as usize)
    }

    pub fn is_proto(&self, name: &str) -> bool {
        self.name2proto.contains_key(name)
    }

    pub fn is_type(&self, name: &str) -> bool {
        self.name2type.contains_key(name)
    }

    fn get_proto_props(&self, name: &str) -> Option<Box<[&Property]>> {
        let p = self.get_proto(name)?;
        let mut propts = Vec::new();

        if !p.parent.is_empty() {
            propts.extend(self.get_props(&p.parent)?);
        }

        propts.extend(p.properties.iter());
        Some(propts.into_boxed_slice())
    }

    fn get_type_props(&self, name: &str) -> Option<Box<[&Property]>> {
        let t = self.get_type(name)?;
        let mut propts = Vec::new();

        if !t.parent.is_empty() {
            propts.extend(self.get_props(&t.parent)?);
        }

        propts.extend(t.properties.iter());
        Some(propts.into_boxed_slice())
    }

    pub fn get_props(&self, name: &str) -> Option<Box<[&Property]>> {
        if self.is_proto(name) {
            self.get_proto_props(name)
        } else if self.is_type(name) {
            self.get_type_props(name)
        } else {
            None
        }
    }

    pub fn get_doc_link(&self, name: &str) -> Option<String> {
        if self.is_proto(name) {
            Some(format!("{}/prototypes/{name}.html", self.base_link))
        } else if let Some(t) = self.get_type(name) {
            if t.inline {
                return None;
            }

            Some(format!("{}/types/{name}.html", self.base_link))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum CurrentType {
    #[default]
    Unknown,
    DataRaw,
    TypeOrProto(Arc<str>),
    BuiltIn(Arc<str>),
    Complex(ComplexType),
}

impl CurrentType {
    fn traverse_prop_internal(&self, helper: &TypeHelper, prop: &str) -> Option<Self> {
        let res = match &self {
            Self::DataRaw => {
                let proto = helper.docs.get_proto_by_type(prop)?;
                Self::Complex(ComplexType::Dictionary {
                    key: Type::Simple("string".into()),
                    value: Type::Simple(proto.name.clone()),
                })
            }
            Self::TypeOrProto(name) => {
                if let Some(&p) = helper
                    .docs
                    .get_props(name)?
                    .iter()
                    .find(|&&p| p.name == prop)
                {
                    helper.type_traverse_helper(&p.type_)
                } else if let Some(custom_p) = &helper.docs.get_proto(name)?.custom_properties {
                    if custom_p.key_type.as_simple()? != "string" {
                        return None;
                    }

                    helper.type_traverse_helper(&custom_p.value_type)
                } else {
                    return None;
                }
            }
            Self::Complex(ComplexType::Dictionary { key, value }) => {
                let key = key.as_simple()?;
                if key != "string" && helper.docs.get_type(&key)?.type_.as_simple()? != "string" {
                    return None;
                }

                helper.type_traverse_helper(value)
            }
            _ => return None,
        };

        Some(res)
    }

    fn traverse_idx_internal(&self, helper: &TypeHelper, idx: usize, len: usize) -> Option<Self> {
        let res = match &self {
            Self::TypeOrProto(name) => {
                let t = helper.docs.get_type(name)?;
                let Type::Complex(c) = &t.type_ else {
                    return None;
                };

                Self::Complex(*c.clone()).traverse_idx_internal(helper, idx, len)?
            }
            Self::Complex(ComplexType::Array { value }) => helper.type_traverse_helper(value),
            Self::Complex(ComplexType::Tuple { values }) => {
                let value = values.get(idx)?;
                helper.type_traverse_helper(value)
            }
            Self::Complex(ComplexType::Union { options, .. }) => {
                let arr = options
                    .iter()
                    .filter_map(|o| {
                        o.as_complex().and_then(|a| {
                            a.as_array()
                                .map(|a| ComplexType::Array { value: a })
                                .or_else(|| {
                                    a.as_tuple().and_then(|t| {
                                        if t.len() == len {
                                            Some(ComplexType::Tuple { values: t })
                                        } else {
                                            None
                                        }
                                    })
                                })
                        })
                    })
                    .collect::<Box<_>>();

                // for now we'll only search for a single array and use that
                if arr.len() != 1 {
                    return None;
                }

                Self::Complex(arr[0].clone()).traverse_idx_internal(helper, idx, len)?
            }
            _ => return None,
        };

        Some(res)
    }
}

#[derive(Clone)]
pub struct TypeHelper {
    docs: Arc<DocHelper>,
    pub kind: CurrentType,
}

impl TypeHelper {
    pub fn new(docs: PrototypeDoc) -> Self {
        let helper = DocHelper::new(docs);
        Self {
            docs: Arc::new(helper),
            kind: CurrentType::DataRaw,
        }
    }

    fn clone_with_kind(&self, kind: CurrentType) -> Self {
        Self {
            docs: Arc::clone(&self.docs),
            kind,
        }
    }

    pub fn traverse_prop(&self, prop: &str) -> Self {
        let kind = self
            .kind
            .traverse_prop_internal(self, prop)
            .unwrap_or(CurrentType::Unknown);

        self.clone_with_kind(kind)
    }

    pub fn traverse_idx(&self, idx: usize, len: usize) -> Self {
        let kind = self
            .kind
            .traverse_idx_internal(self, idx, len)
            .unwrap_or(CurrentType::Unknown);

        self.clone_with_kind(kind)
    }

    fn type_traverse_helper(&self, t: &Type) -> CurrentType {
        use CurrentType::{BuiltIn, TypeOrProto};
        use Type::{Complex, Simple};

        match t {
            Simple(t) => {
                let tt = t.clone().into();

                let Some(t_info) = self.docs.get_type(t) else {
                    return TypeOrProto(tt);
                };

                if t_info.type_ == Simple("builtin".into()) {
                    BuiltIn(tt)
                } else {
                    TypeOrProto(tt)
                }
            }
            Complex(c) => CurrentType::Complex(*c.clone()),
        }
    }

    pub fn get_doc_link(&self, name: Arc<str>) -> Option<String> {
        self.docs.get_doc_link(&name)
    }
}
