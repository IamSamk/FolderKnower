//! Data models for the ER view: ORM tables, DTO/validation schemas, Prisma models and
//! SQL DDL tables, plus relations, schema<->table mirrors and function->table edges.

use std::collections::{BTreeMap, BTreeSet};

use ark_link::{ParsedFile, file_id, fn_id};
use ark_parse::{ClassFact, FieldFact};
use ark_scan::Language;
use ark_schema::{
    CrossTierEdge, DataField, DataModel, DataModelKind, Protocol, Relation, RelationKind,
};
use regex::Regex;

fn model_id(path: &str, name: &str) -> String {
    format!("model:{path}#{name}")
}

fn has(s: &Option<String>, needle: &str) -> bool {
    s.as_deref().is_some_and(|v| v.contains(needle))
}

fn first_string_arg(v: &str, func: &str) -> Option<String> {
    let i = v.find(func)? + func.len();
    let rest = &v[i..];
    let rest = rest.trim_start_matches('(').trim_start();
    let q = rest.chars().next()?;
    if q == '"' || q == '\'' {
        rest[1..].split(q).next().map(str::to_owned)
    } else {
        // ForeignKey(User, ...) / relationship(User)
        let ident: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '.')
            .collect();
        (!ident.is_empty()).then_some(ident)
    }
}

fn first_arg_ident(v: &str, func: &str) -> Option<String> {
    let i = v.find(func)? + func.len();
    let rest = v[i..].trim_start_matches('(');
    let t: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '.')
        .collect();
    (!t.is_empty()).then_some(t)
}

const SKIP_FIELDS: &[&str] = &[
    "__tablename__",
    "__table_args__",
    "model_config",
    "Config",
    "Meta",
    "objects",
    "__abstract__",
    "__mapper_args__",
];

fn python_model(c: &ClassFact) -> Option<(DataModelKind, &'static str)> {
    let bases = c.bases.join(",");
    let column_like = c.fields.iter().any(|f| {
        has(&f.value, "Column(")
            || has(&f.value, "mapped_column(")
            || has(&f.value, "relationship(")
    }) || c.fields.iter().any(|f| f.name == "__tablename__");
    if bases.contains("models.Model") {
        return Some((DataModelKind::Table, "django"));
    }
    if bases.contains("SQLModel") {
        let table = c.fields.iter().any(|f| has(&f.value, "primary_key"));
        return Some((
            if table {
                DataModelKind::Table
            } else {
                DataModelKind::Schema
            },
            "sqlmodel",
        ));
    }
    if column_like {
        return Some((DataModelKind::Table, "sqlalchemy"));
    }
    if bases.contains("BaseModel") || bases.contains("Schema") || bases.contains("TypedDict") {
        return Some((DataModelKind::Schema, "pydantic"));
    }
    if c.decorators.iter().any(|d| d.contains("dataclass")) && !c.fields.is_empty() {
        return Some((DataModelKind::Schema, "dataclass"));
    }
    if (bases.contains("Document") || bases.contains("EmbeddedDocument")) && !c.fields.is_empty() {
        return Some((DataModelKind::Document, "mongoengine"));
    }
    None
}

fn strip_mapped(t: &str) -> String {
    let t = t.trim();
    if let Some(inner) = t.strip_prefix("Mapped[").and_then(|x| x.strip_suffix(']')) {
        inner.to_owned()
    } else {
        t.to_owned()
    }
}

fn python_field(
    f: &FieldFact,
    relations: &mut Vec<(String, RelationKind, String)>,
) -> Option<DataField> {
    if SKIP_FIELDS.contains(&f.name.as_str()) || f.name.starts_with("__") {
        return None;
    }
    let v = f.value.clone().unwrap_or_default();
    if v.contains("relationship(") {
        if let Some(t) = first_string_arg(&v, "relationship") {
            let kind = if v.contains("uselist=False") {
                RelationKind::OneToOne
            } else if f.name.ends_with('s')
                || f.ty
                    .as_deref()
                    .is_some_and(|t| t.contains("list") || t.contains("List"))
            {
                RelationKind::OneToMany
            } else {
                RelationKind::ManyToOne
            };
            relations.push((t, kind, f.name.clone()));
        }
        return None;
    }
    let mut ty = f.ty.as_deref().map(strip_mapped);
    for func in ["Column", "mapped_column", "Field"] {
        if ty.is_none()
            && let Some(t) = first_arg_ident(&v, func)
            && t.chars().next().is_some_and(|c| c.is_uppercase())
            && t != "ForeignKey"
        {
            ty = Some(t);
        }
    }
    if ty.is_none()
        && let Some(i) = v.find("models.")
    {
        ty = Some(
            v[i + 7..]
                .chars()
                .take_while(|c| c.is_alphanumeric())
                .collect(),
        );
    }
    let mut references = None;
    if v.contains("ForeignKey(") {
        references = first_string_arg(&v, "ForeignKey");
        if let Some(r) = &references {
            relations.push((r.clone(), RelationKind::ManyToOne, f.name.clone()));
        }
    }
    for (func, kind) in [
        ("ManyToManyField(", RelationKind::ManyToMany),
        ("OneToOneField(", RelationKind::OneToOne),
    ] {
        if v.contains(func)
            && let Some(r) = first_string_arg(&v, func.trim_end_matches('('))
        {
            relations.push((r.clone(), kind, f.name.clone()));
            references = Some(r);
        }
    }
    let nullable = v.contains("nullable=True")
        || v.contains("null=True")
        || ty
            .as_deref()
            .is_some_and(|t| t.contains("Optional") || t.contains("None"));
    Some(DataField {
        name: f.name.clone(),
        ty,
        primary_key: v.contains("primary_key=True") || v.contains("primary_key = True"),
        nullable,
        references,
    })
}

fn table_name_py(c: &ClassFact) -> Option<String> {
    c.fields
        .iter()
        .find(|f| f.name == "__tablename__")
        .and_then(|f| f.value.as_deref())
        .map(|v| v.trim_matches(['"', '\'']).to_owned())
}

fn js_model(c: &ClassFact) -> Option<(DataModelKind, &'static str)> {
    if c.bases.iter().any(|b| b == "zod") {
        return Some((DataModelKind::Schema, "zod"));
    }
    if c.decorators.iter().any(|d| d.starts_with("@Entity")) {
        return Some((DataModelKind::Table, "typeorm"));
    }
    if c.decorators.iter().any(|d| d.starts_with("@Table")) || c.bases.iter().any(|b| b == "Model")
    {
        return Some((DataModelKind::Table, "sequelize"));
    }
    if c.decorators.iter().any(|d| d.starts_with("@Schema")) {
        return Some((DataModelKind::Document, "mongoose"));
    }
    let dto = ["Dto", "DTO", "Input", "Request", "Payload"];
    if dto.iter().any(|s| c.name.ends_with(s)) && !c.fields.is_empty() {
        return Some((DataModelKind::Schema, "dto"));
    }
    None
}

fn js_field(f: &FieldFact, relations: &mut Vec<(String, RelationKind, String)>) -> DataField {
    let v = f.value.clone().unwrap_or_default();
    for (dec, kind) in [
        ("@ManyToOne", RelationKind::ManyToOne),
        ("@OneToMany", RelationKind::OneToMany),
        ("@OneToOne", RelationKind::OneToOne),
        ("@ManyToMany", RelationKind::ManyToMany),
    ] {
        if let Some(i) = v.find(dec) {
            let rest = &v[i..];
            if let Some(j) = rest.find("=>") {
                let t: String = rest[j + 2..]
                    .trim_start()
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !t.is_empty() {
                    relations.push((t, kind, f.name.clone()));
                }
            }
        }
    }
    DataField {
        name: f.name.clone(),
        ty: f.ty.clone(),
        primary_key: v.contains("Primary") || f.name == "id" && v.contains("@"),
        nullable: v.contains("nullable: true")
            || f.ty.as_deref().is_some_and(|t| {
                t.contains("null") || t.contains("undefined") || t.contains(".optional()")
            }),
        references: None,
    }
}

fn rust_model(c: &ClassFact) -> Option<(DataModelKind, &'static str)> {
    let d = c.decorators.join(" ");
    if d.contains("FromRow") {
        return Some((DataModelKind::Table, "sqlx"));
    }
    if d.contains("Queryable") || d.contains("Insertable") || d.contains("diesel(") {
        return Some((DataModelKind::Table, "diesel"));
    }
    if d.contains("DeriveEntityModel") {
        return Some((DataModelKind::Table, "sea-orm"));
    }
    let dto = [
        "Request", "Payload", "Input", "Dto", "Body", "Response", "Params", "Query", "Form",
    ];
    if d.contains("Deserialize") && dto.iter().any(|s| c.name.ends_with(s)) {
        return Some((DataModelKind::Schema, "serde"));
    }
    None
}

fn go_model(c: &ClassFact) -> Option<(DataModelKind, &'static str)> {
    if c.bases.iter().any(|b| b.contains("gorm.Model"))
        || c.fields.iter().any(|f| has(&f.value, "gorm:"))
    {
        return Some((DataModelKind::Table, "gorm"));
    }
    if c.fields.iter().any(|f| has(&f.value, "db:\"")) {
        return Some((DataModelKind::Table, "sqlx"));
    }
    if c.fields.iter().any(|f| has(&f.value, "bson:\"")) {
        return Some((DataModelKind::Document, "mongo"));
    }
    let dto = ["Request", "Response", "Payload", "Input", "DTO", "Dto"];
    if dto.iter().any(|s| c.name.ends_with(s)) && c.fields.iter().any(|f| has(&f.value, "json:\""))
    {
        return Some((DataModelKind::Schema, "json"));
    }
    None
}

/// Models declared in source code.
pub fn code_models(files: &[ParsedFile]) -> Vec<(DataModel, Vec<(String, RelationKind, String)>)> {
    let mut out = Vec::new();
    for f in files {
        let Some(lang) = f.scan.language else {
            continue;
        };
        let path = &f.scan.path;
        for c in &f.facts.classes {
            let detected = match lang {
                Language::Python => python_model(c),
                Language::TypeScript | Language::Tsx | Language::JavaScript => js_model(c),
                Language::Rust => rust_model(c),
                Language::Go => go_model(c),
                Language::Java | Language::CSharp => None,
            };
            let Some((kind, framework)) = detected else {
                continue;
            };
            let mut rels = Vec::new();
            let fields: Vec<DataField> = match lang {
                Language::Python => c
                    .fields
                    .iter()
                    .filter_map(|x| python_field(x, &mut rels))
                    .collect(),
                Language::TypeScript | Language::Tsx | Language::JavaScript => {
                    c.fields.iter().map(|x| js_field(x, &mut rels)).collect()
                }
                _ => c
                    .fields
                    .iter()
                    .map(|x| DataField {
                        name: x.name.clone(),
                        ty: x.ty.clone(),
                        primary_key: x.name.eq_ignore_ascii_case("id")
                            || has(&x.value, "primaryKey")
                            || has(&x.value, "primary_key"),
                        nullable: x
                            .ty
                            .as_deref()
                            .is_some_and(|t| t.starts_with("Option<") || t.starts_with('*')),
                        references: None,
                    })
                    .collect(),
            };
            let table = match lang {
                Language::Python => table_name_py(c),
                Language::TypeScript | Language::Tsx | Language::JavaScript => c
                    .decorators
                    .iter()
                    .find(|d| d.starts_with("@Entity"))
                    .and_then(|d| first_string_arg(d, "@Entity")),
                _ => None,
            };
            out.push((
                DataModel {
                    id: model_id(path, &c.name),
                    name: c.name.clone(),
                    kind,
                    framework: framework.to_owned(),
                    file_id: file_id(path),
                    line: c.line,
                    table,
                    fields,
                    relations: Vec::new(),
                    mirrors: Vec::new(),
                },
                rels,
            ));
        }
    }
    out
}

/// Prisma `model X { ... }` blocks.
pub fn prisma_models(
    path: &str,
    src: &str,
) -> Vec<(DataModel, Vec<(String, RelationKind, String)>)> {
    let re = Regex::new(r"(?m)^\s*model\s+(\w+)\s*\{([^}]*)\}").expect("regex");
    let mut out = Vec::new();
    for cap in re.captures_iter(src) {
        let name = cap[1].to_owned();
        let line = src[..cap.get(0).map(|m| m.start()).unwrap_or(0)]
            .lines()
            .count() as u32
            + 1;
        let mut fields = Vec::new();
        let mut rels = Vec::new();
        let mut table = None;
        for l in cap[2].lines() {
            let l = l.trim();
            if l.is_empty() || l.starts_with("//") {
                continue;
            }
            if let Some(m) = l.strip_prefix("@@map(") {
                table = m.trim_end_matches(')').trim_matches('"').to_owned().into();
                continue;
            }
            if l.starts_with("@@") {
                continue;
            }
            let mut parts = l.split_whitespace();
            let (Some(fname), Some(fty)) = (parts.next(), parts.next()) else {
                continue;
            };
            let base = fty.trim_end_matches(['?', ']', '[']);
            let is_model_ref = base.chars().next().is_some_and(|c| c.is_uppercase())
                && !matches!(
                    base,
                    "String"
                        | "Int"
                        | "BigInt"
                        | "Float"
                        | "Decimal"
                        | "Boolean"
                        | "DateTime"
                        | "Json"
                        | "Bytes"
                );
            if is_model_ref {
                let kind = if fty.ends_with("[]") {
                    RelationKind::OneToMany
                } else {
                    RelationKind::ManyToOne
                };
                // Only the side with `fields:` owns the FK; list sides are informational.
                if kind == RelationKind::OneToMany || l.contains("fields:") {
                    rels.push((base.to_owned(), kind, fname.to_owned()));
                }
                continue;
            }
            fields.push(DataField {
                name: fname.to_owned(),
                ty: Some(fty.trim_end_matches('?').to_owned()),
                primary_key: l.contains("@id"),
                nullable: fty.ends_with('?'),
                references: None,
            });
        }
        out.push((
            DataModel {
                id: model_id(path, &name),
                name,
                kind: DataModelKind::Table,
                framework: "prisma".to_owned(),
                file_id: file_id(path),
                line,
                table,
                fields,
                relations: Vec::new(),
                mirrors: Vec::new(),
            },
            rels,
        ));
    }
    out
}

/// Protobuf `message X { ... }` blocks (top level and nested, flattened).
pub fn proto_models(
    path: &str,
    src: &str,
) -> Vec<(DataModel, Vec<(String, RelationKind, String)>)> {
    let head = Regex::new(r"(?m)^\s*message\s+(\w+)\s*\{").expect("regex");
    let field = Regex::new(r"^(?:(repeated|optional)\s+)?([\w.]+|map<[^>]+>)\s+(\w+)\s*=\s*\d+")
        .expect("regex");
    let scalar = [
        "double", "float", "int32", "int64", "uint32", "uint64", "sint32", "sint64", "fixed32",
        "fixed64", "sfixed32", "sfixed64", "bool", "string", "bytes",
    ];
    let mut out = Vec::new();
    for cap in head.captures_iter(src) {
        let start = cap.get(0).map(|m| m.end()).unwrap_or(0);
        let mut depth = 1usize;
        let mut end = start;
        for (i, ch) in src[start..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = start + i;
                        break;
                    }
                }
                _ => {}
            }
        }
        if depth != 0 {
            continue;
        }
        let name = cap[1].to_owned();
        let line = src[..cap.get(0).map(|m| m.start()).unwrap_or(0)]
            .lines()
            .count() as u32
            + 1;
        let mut fields = Vec::new();
        let mut rels = Vec::new();
        let mut nest = 0usize;
        for l in src[start..end].lines() {
            let l = l.trim();
            if l.starts_with("//") || l.is_empty() {
                continue;
            }
            // Skip lines inside nested message/enum/oneof-free blocks.
            let opens = l.matches('{').count();
            let closes = l.matches('}').count();
            if nest > 0 || (opens > closes && !l.starts_with("oneof")) {
                nest = (nest + opens).saturating_sub(closes);
                continue;
            }
            let Some(f) = field.captures(l) else { continue };
            let ty = f[2].to_owned();
            let repeated = f.get(1).is_some_and(|m| m.as_str() == "repeated");
            let base = ty.rsplit('.').next().unwrap_or(&ty).to_owned();
            let is_ref = !scalar.contains(&ty.as_str())
                && !ty.starts_with("map<")
                && base.chars().next().is_some_and(|c| c.is_uppercase());
            if is_ref {
                let kind = if repeated {
                    RelationKind::OneToMany
                } else {
                    RelationKind::ManyToOne
                };
                rels.push((base, kind, f[3].to_owned()));
                continue;
            }
            fields.push(DataField {
                name: f[3].to_owned(),
                ty: Some(ty),
                primary_key: false,
                nullable: f.get(1).is_some_and(|m| m.as_str() == "optional"),
                references: None,
            });
        }
        out.push((
            DataModel {
                id: model_id(path, &name),
                name,
                kind: DataModelKind::Schema,
                framework: "protobuf".to_owned(),
                file_id: file_id(path),
                line,
                table: None,
                fields,
                relations: Vec::new(),
                mirrors: Vec::new(),
            },
            rels,
        ));
    }
    out
}

/// `CREATE TABLE x (...)` statements.
pub fn sql_models(path: &str, src: &str) -> Vec<(DataModel, Vec<(String, RelationKind, String)>)> {
    let re = Regex::new(r#"(?is)create\s+table\s+(?:if\s+not\s+exists\s+)?[`"\[]?(?:\w+\.)?(\w+)[`"\]]?\s*\((.*?)\)\s*;"#)
        .expect("regex");
    let fk =
        Regex::new(r#"(?i)references\s+[`"\[]?(\w+)[`"\]]?\s*\(\s*[`"]?(\w+)"#).expect("regex");
    let tfk = Regex::new(r#"(?i)foreign\s+key\s*\(\s*[`"]?(\w+)[`"]?\s*\)\s*references\s+[`"\[]?(\w+)[`"\]]?\s*\(\s*[`"]?(\w+)"#).expect("regex");
    let mut out = Vec::new();
    for cap in re.captures_iter(src) {
        let table = cap[1].to_owned();
        let line = src[..cap.get(0).map(|m| m.start()).unwrap_or(0)]
            .lines()
            .count() as u32
            + 1;
        let mut fields: Vec<DataField> = Vec::new();
        let mut rels = Vec::new();
        // Split on commas not inside parentheses.
        let body = &cap[2];
        let mut depth = 0;
        let mut cur = String::new();
        let mut cols = Vec::new();
        for ch in body.chars() {
            match ch {
                '(' => depth += 1,
                ')' => depth -= 1,
                ',' if depth == 0 => {
                    cols.push(std::mem::take(&mut cur));
                    continue;
                }
                _ => {}
            }
            cur.push(ch);
        }
        cols.push(cur);
        for c in cols {
            let c = c.trim();
            let lower = c.to_ascii_lowercase();
            if let Some(m) = tfk.captures(c) {
                let target = format!("{}.{}", &m[2], &m[3]);
                if let Some(f) = fields.iter_mut().find(|f| f.name == m[1]) {
                    f.references = Some(target.clone());
                }
                rels.push((target, RelationKind::ManyToOne, m[1].to_owned()));
                continue;
            }
            if lower.starts_with("primary key") {
                for name in c[c.find('(').unwrap_or(0)..].split([',', '(', ')', ' ', '`', '"']) {
                    if let Some(f) = fields.iter_mut().find(|f| f.name == name) {
                        f.primary_key = true;
                    }
                }
                continue;
            }
            if lower.starts_with("constraint")
                || lower.starts_with("unique")
                || lower.starts_with("index")
                || lower.starts_with("key ")
                || lower.starts_with("check")
            {
                continue;
            }
            let mut parts = c.split_whitespace();
            let (Some(name), ty) = (parts.next(), parts.next()) else {
                continue;
            };
            let name = name.trim_matches(['`', '"', '[', ']']).to_owned();
            let references = fk.captures(c).map(|m| format!("{}.{}", &m[1], &m[2]));
            if let Some(r) = &references {
                rels.push((r.clone(), RelationKind::ManyToOne, name.clone()));
            }
            fields.push(DataField {
                name,
                ty: ty.map(|t| t.to_ascii_uppercase()),
                primary_key: lower.contains("primary key"),
                nullable: !lower.contains("not null") && !lower.contains("primary key"),
                references,
            });
        }
        out.push((
            DataModel {
                id: model_id(path, &table),
                name: table.clone(),
                kind: DataModelKind::Table,
                framework: "sql".to_owned(),
                file_id: file_id(path),
                line,
                table: Some(table),
                fields,
                relations: Vec::new(),
                mirrors: Vec::new(),
            },
            rels,
        ));
    }
    out
}

fn base_name(name: &str) -> String {
    let mut n = name.to_owned();
    const SUFFIXES: &[&str] = &[
        "Create", "Update", "In", "Out", "Read", "Base", "Schema", "Dto", "DTO", "Request",
        "Response", "Public", "Input", "Payload", "Model", "Entity", "Record", "Row", "Table",
        "Detail", "List", "Patch",
    ];
    const PREFIXES: &[&str] = &["Create", "Update", "New", "Patch"];
    loop {
        let before = n.clone();
        for s in SUFFIXES {
            if n.len() > s.len() + 1 && n.ends_with(s) {
                n.truncate(n.len() - s.len());
            }
        }
        for p in PREFIXES {
            if n.len() > p.len() + 1 && n.starts_with(p) {
                n = n[p.len()..].to_owned();
            }
        }
        if n == before {
            break;
        }
    }
    let l = n.to_ascii_lowercase();
    l.strip_suffix("ies")
        .map(|s| format!("{s}y"))
        .or_else(|| l.strip_suffix('s').map(str::to_owned))
        .unwrap_or(l)
}

/// Resolve relations to model ids and compute schema<->table mirrors.
pub fn finalize(mut raw: Vec<(DataModel, Vec<(String, RelationKind, String)>)>) -> Vec<DataModel> {
    raw.sort_by(|a, b| a.0.id.cmp(&b.0.id));
    raw.dedup_by(|a, b| a.0.id == b.0.id);
    let lookup: Vec<(String, String, Option<String>, DataModelKind)> = raw
        .iter()
        .map(|(m, _)| (m.id.clone(), m.name.clone(), m.table.clone(), m.kind))
        .collect();
    let find = |target: &str| -> Option<String> {
        let t = target.split('.').next().unwrap_or(target);
        let tl = t.to_ascii_lowercase();
        let tb = base_name(t);
        let pick = |pred: &dyn Fn(&(String, String, Option<String>, DataModelKind)) -> bool| {
            lookup
                .iter()
                .filter(|m| pred(m))
                .map(|m| m.0.clone())
                .next()
        };
        pick(&|m| {
            m.3 != DataModelKind::Schema
                && m.2.as_deref().is_some_and(|x| x.eq_ignore_ascii_case(t))
        })
        .or_else(|| pick(&|m| m.3 != DataModelKind::Schema && m.1 == t))
        .or_else(|| pick(&|m| m.3 != DataModelKind::Schema && m.1.to_ascii_lowercase() == tl))
        .or_else(|| pick(&|m| m.3 != DataModelKind::Schema && base_name(&m.1) == tb))
    };
    let mut models: Vec<DataModel> = raw
        .into_iter()
        .map(|(mut m, rels)| {
            let mut seen = BTreeSet::new();
            for (target, kind, via) in rels {
                if let Some(id) = find(&target)
                    && seen.insert((id.clone(), via.clone()))
                {
                    m.relations.push(Relation {
                        target: id,
                        kind,
                        via,
                    });
                }
            }
            m.relations
                .sort_by(|a, b| (&a.target, &a.via).cmp(&(&b.target, &b.via)));
            // `owner = relationship("User")` next to `owner_id = ForeignKey(...)` is one
            // relation; keep the side backed by a real column.
            let cols: BTreeSet<String> = m.fields.iter().map(|f| f.name.clone()).collect();
            let backed: BTreeSet<(String, RelationKind)> = m
                .relations
                .iter()
                .filter(|r| cols.contains(&r.via))
                .map(|r| (r.target.clone(), r.kind))
                .collect();
            m.relations
                .retain(|r| cols.contains(&r.via) || !backed.contains(&(r.target.clone(), r.kind)));
            m
        })
        .collect();
    // Mirrors: same base name, or >=60% of a schema's fields present on a table.
    let snapshot: Vec<(String, String, DataModelKind, BTreeSet<String>)> = models
        .iter()
        .map(|m| {
            (
                m.id.clone(),
                base_name(&m.name),
                m.kind,
                m.fields
                    .iter()
                    .map(|f| f.name.to_ascii_lowercase())
                    .collect(),
            )
        })
        .collect();
    for m in &mut models {
        let me = snapshot.iter().find(|s| s.0 == m.id).expect("present");
        let mut mirrors: Vec<String> = snapshot
            .iter()
            .filter(|o| {
                o.0 != me.0 && (o.2 == DataModelKind::Schema) != (me.2 == DataModelKind::Schema)
            })
            .filter(|o| {
                if o.1 == me.1 {
                    return true;
                }
                let (schema, table) = if me.2 == DataModelKind::Schema {
                    (&me.3, &o.3)
                } else {
                    (&o.3, &me.3)
                };
                let shared = schema.intersection(table).count();
                schema.len() >= 2 && shared >= 2 && shared * 10 >= schema.len() * 8
            })
            .map(|o| o.0.clone())
            .collect();
        mirrors.sort();
        m.mirrors = mirrors;
    }
    models
}

/// Function -> table edges from ORM usage (`Todo(...)`, `Todo.objects`, `select(Todo)`,
/// `prisma.todo.findMany`, `db.query(Todo)`).
pub fn db_edges(files: &[ParsedFile], models: &[DataModel]) -> Vec<CrossTierEdge> {
    let tables: Vec<&DataModel> = models
        .iter()
        .filter(|m| m.kind != DataModelKind::Schema)
        .collect();
    let by_name: BTreeMap<String, &DataModel> = tables
        .iter()
        .flat_map(|m| {
            let mut v = vec![(m.name.clone(), *m), (m.name.to_ascii_lowercase(), *m)];
            if let Some(t) = &m.table {
                v.push((t.to_ascii_lowercase(), *m));
            }
            v
        })
        .collect();
    let mut out: BTreeMap<String, CrossTierEdge> = BTreeMap::new();
    for f in files {
        for func in &f.facts.functions {
            let src = fn_id(&f.scan.path, &func.qualname);
            for c in &func.calls {
                let mut hit: Option<(&DataModel, String)> = None;
                if let Some(m) = by_name.get(&c.callee)
                    && c.callee.chars().next().is_some_and(|x| x.is_uppercase())
                {
                    hit = Some((m, format!("orm.construct {}", c.callee)));
                }
                if hit.is_none()
                    && let Some(r) = &c.receiver
                {
                    let segs: Vec<&str> = r.split(['.', ':']).filter(|s| !s.is_empty()).collect();
                    for s in &segs {
                        let key = if s.chars().next().is_some_and(|x| x.is_uppercase()) {
                            s.to_string()
                        } else if segs
                            .first()
                            .is_some_and(|f| f.contains("prisma") || *f == "db")
                        {
                            s.to_ascii_lowercase()
                        } else {
                            continue;
                        };
                        if let Some(m) = by_name.get(&key) {
                            hit = Some((m, format!("orm.{}.{}", s, c.callee)));
                            break;
                        }
                    }
                }
                if hit.is_none() {
                    for a in &c.ident_args {
                        let a0 = a.split('.').next().unwrap_or(a);
                        if a0.chars().next().is_some_and(|x| x.is_uppercase())
                            && let Some(m) = by_name.get(a0)
                        {
                            hit = Some((m, format!("orm.{}({a0})", c.callee)));
                            break;
                        }
                    }
                }
                if let Some((m, ev)) = hit {
                    let id = format!("xedge:{src}->{}", m.id);
                    let e = out.entry(id.clone()).or_insert(CrossTierEdge {
                        id,
                        source: src.clone(),
                        target: m.id.clone(),
                        protocol: Protocol::Sql,
                        evidence: vec![format!("orm.{}", m.framework)],
                    });
                    if !e.evidence.contains(&ev) && e.evidence.len() < 6 {
                        e.evidence.push(ev);
                    }
                }
            }
        }
    }
    let mut v: Vec<CrossTierEdge> = out.into_values().collect();
    for e in &mut v {
        e.evidence.sort();
    }
    v
}
