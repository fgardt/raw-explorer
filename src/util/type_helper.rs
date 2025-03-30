use std::{collections::HashMap, sync::Arc};

use fapi_diff::format::prototype::{
    ComplexType, LiteralValue, Property, Prototype, PrototypeDoc, Type, TypeConcept,
};

struct DocHelper {
    docs: PrototypeDoc,

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

        Self {
            docs,
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
}

#[derive(Clone, Default)]
pub enum CurrentType {
    #[default]
    Unknown,
    DataRaw,
    ProtoArray(Arc<str>),
    TypeOrProto(Arc<str>),
    Complex(ComplexType),
}

impl CurrentType {
    pub fn display(&self) -> String {
        match self {
            CurrentType::Unknown => "?".to_string(),
            CurrentType::DataRaw => "data.raw".to_string(),
            CurrentType::ProtoArray(name) => format!("dictionary[string -> {name}]"),
            CurrentType::TypeOrProto(name) => format!("{name}"),
            CurrentType::Complex(complex) => complex_printer(complex),
        }
    }
}

fn complex_printer(complex: &ComplexType) -> String {
    match complex {
        ComplexType::Array { value } => match value {
            Type::Simple(t) => format!("array[{t}]"),
            Type::Complex(c) => format!("array[{}]", complex_printer(c)),
        },
        ComplexType::Dictionary { key, value } => {
            let k = match key {
                Type::Simple(t) => t.clone(),
                Type::Complex(c) => complex_printer(c),
            };
            let v = match value {
                Type::Simple(t) => t.clone(),
                Type::Complex(c) => complex_printer(c),
            };

            format!("dictionary[{k} -> {v}]")
        }
        ComplexType::Tuple { values } => {
            let values = values
                .iter()
                .map(|v| match v {
                    Type::Simple(t) => t.clone(),
                    Type::Complex(c) => complex_printer(c),
                })
                .collect::<Vec<_>>()
                .join(", ");

            format!("({values})")
        }
        ComplexType::Union { options, .. } => {
            let options = options
                .iter()
                .map(|o| match o {
                    Type::Simple(t) => t.clone(),
                    Type::Complex(c) => complex_printer(c),
                })
                .collect::<Vec<_>>()
                .join(" | ");

            format!("union[{options}]")
        }
        ComplexType::Type { value, .. } => match value {
            Type::Simple(t) => t.clone(),
            Type::Complex(c) => complex_printer(c),
        },
        ComplexType::Literal(literal) => match &literal.value {
            LiteralValue::String(s) => s.clone(),
            LiteralValue::UInt(u) => u.to_string(),
            LiteralValue::Int(i) => i.to_string(),
            LiteralValue::Float(f) => f.to_string(),
            LiteralValue::Boolean(b) => b.to_string(),
        },
        ComplexType::Struct => "struct".to_string(),
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

    pub fn traverse_prop(&self, prop: &str) -> Option<Self> {
        match &self.kind {
            CurrentType::DataRaw => {
                let proto = self.docs.get_proto_by_type(prop)?;
                let kind = CurrentType::ProtoArray(proto.name.clone().into());
                Some(self.clone_with_kind(kind))
            }
            CurrentType::ProtoArray(name) => {
                let kind = CurrentType::TypeOrProto(name.clone());
                Some(self.clone_with_kind(kind))
            }
            CurrentType::TypeOrProto(name) => {
                let p = *self
                    .docs
                    .get_props(name)?
                    .iter()
                    .find(|&&p| p.name == prop)?;

                let kind = match p.type_ {
                    Type::Simple(ref t) => CurrentType::TypeOrProto(t.clone().into()),
                    Type::Complex(ref c) => CurrentType::Complex(*c.clone()),
                };

                Some(self.clone_with_kind(kind))
            }
            _ => None,
        }
    }

    pub fn traverse_idx(&self, idx: usize) -> Option<Self> {
        let CurrentType::Complex(complex) = &self.kind else {
            return None;
        };

        let kind = match complex {
            ComplexType::Array { value } => match value {
                Type::Simple(t) => CurrentType::TypeOrProto(t.clone().into()),
                Type::Complex(c) => CurrentType::Complex(*c.clone()),
            },
            ComplexType::Tuple { values } => {
                let value = values.get(idx)?;
                match value {
                    Type::Simple(t) => CurrentType::TypeOrProto(t.clone().into()),
                    Type::Complex(c) => CurrentType::Complex(*c.clone()),
                }
            }
            _ => return None,
        };

        Some(self.clone_with_kind(kind))
    }
}
