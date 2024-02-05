use std::{collections::VecDeque, fs, path::PathBuf, process::Command, str::FromStr};

use crate::{
    core::{math::Vec2, structures::ErrorCapability},
    editor_state::windows::GraspEditorWindow,
};
use itertools::Itertools;
use mosaic::{
    capabilities::ArchetypeSubject,
    internals::{Tile, TileFieldSetter, S32},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ASTCaret {
    pub offset: Option<u32>,
    pub line: Option<u32>,
    pub col: Option<u32>,
    #[serde(alias = "tokLen")]
    pub tok_len: Option<u32>,
    pub file: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ASTRange {
    pub begin: ASTCaret,
    pub end: ASTCaret,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ASTType {
    #[serde(alias = "desugaredQualType")]
    #[serde(default)]
    pub desugared_qual_type: String,
    #[serde(alias = "qualType")]
    #[serde(default)]
    pub qual_type: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ASTTor {
    #[serde(default)]
    #[serde(alias = "hasConstParam")]
    pub has_const_param: bool,
    #[serde(default)]
    #[serde(alias = "implicitHasConstParam")]
    pub implicit_has_const_param: bool,
    #[serde(default)]
    #[serde(alias = "needsImplicit")]
    pub needs_implicit: bool,
    #[serde(default)]
    pub simple: bool,
    #[serde(default)]
    pub trivial: bool,
    #[serde(default)]
    pub exists: bool,
    #[serde(default)]
    pub irrelevant: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ASTDefinition {
    #[serde(default)]
    #[serde(alias = "isAggregate")]
    pub is_aggregate: bool,
    #[serde(default)]
    #[serde(alias = "isLiteral")]
    pub is_literal: bool,
    #[serde(default)]
    #[serde(alias = "isPOD")]
    pub is_pod: bool,
    #[serde(default)]
    #[serde(alias = "isStandardLayout")]
    pub is_standard_layout: bool,
    #[serde(default)]
    #[serde(alias = "isTrivial")]
    pub is_trivial: bool,
    #[serde(default)]
    #[serde(alias = "isTriviallyCopyable")]
    pub is_trivially_copyable: bool,
    #[serde(default)]
    #[serde(alias = "copyAssign")]
    pub copy_assign: ASTTor,
    #[serde(default)]
    #[serde(alias = "copyCtor")]
    pub copy_ctor: ASTTor,
    #[serde(default)]
    #[serde(alias = "moveAssign")]
    pub move_assign: ASTTor,
    #[serde(default)]
    #[serde(alias = "moveCtor")]
    pub move_ctor: ASTTor,
    #[serde(default)]
    #[serde(alias = "defaultCtor")]
    pub default_ctor: ASTTor,
    #[serde(default)]
    #[serde(alias = "dtor")]
    pub dtor: ASTTor,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ASTNode {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    #[serde(alias = "tagUsed")]
    pub tag_used: String,
    #[serde(default)]
    pub kind: String,
    pub loc: Option<ASTCaret>,
    pub range: Option<ASTRange>,
    #[serde(default)]
    #[serde(alias = "isUsed")]
    pub is_used: bool,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    #[serde(alias = "type")]
    pub typ: ASTType,
    #[serde(default)]
    pub init: String,
    #[serde(default)]
    #[serde(alias = "valueCategory")]
    pub value_category: String,
    #[serde(default)]
    pub value: Value,
    #[serde(default)]
    pub opcode: Option<String>,
    #[serde(default)]
    #[serde(alias = "castKind")]
    pub cast_kind: String,
    #[serde(default)]
    pub inner: Vec<ASTNode>,
    #[serde(default)]
    #[serde(alias = "isReferenced")]
    pub is_refd: bool,
    #[serde(default)]
    #[serde(alias = "referencedDecl")]
    pub ref_decl: Option<Box<ASTNode>>,
    #[serde(default)]
    #[serde(alias = "definitionData")]
    pub definition_data: Option<ASTDefinition>,
    #[serde(default)]
    #[serde(alias = "completeDefinition")]
    pub complete_definition: bool,
}

fn ast_recursive_descent(
    window: &mut GraspEditorWindow,
    job: &ASTNode,
    parent: Option<&Tile>,
    mut pos: Vec2,
    mut layer: f32,
) -> Option<Tile> {
    let (should_create, parent_connect) = match job.kind.as_str() {
        "CompoundStmt" | "ImplicitCastExpr" => (false, parent.unwrap().clone()),
        _ => (true, window.create_new_object(pos)),
    };

    if should_create {
        let label = match job.kind.as_str() {
            "StringLiteral" => {
                let l = job.value.as_str().map(|s| s.to_string()).unwrap();
                if l.len() > 30 {
                    l[0..28].to_string() + "..."
                } else {
                    l
                }
            }
            .to_string(),
            "VarDecl" => format!("{}: {}", job.name, job.typ.qual_type),
            "BinaryOperator" => format!(
                "{}.{}",
                job.typ.qual_type,
                job.opcode.clone().unwrap_or_default()
            ),
            "UnaryOperator" => format!(
                "{}.unary_{}",
                job.typ.qual_type,
                job.opcode.clone().unwrap_or_default()
            ),
            "UnresolvedLookupExpr" => job.name.to_string(),
            "FunctionDecl" => format!("{}: {}", job.name, job.typ.qual_type),
            "IntegerLiteral" => format!("{}: {}", job.value, job.typ.qual_type),
            "CompoundAssignOperator" => format!(
                "{}.{}",
                job.typ.qual_type,
                job.opcode.clone().unwrap_or_default()
            ),
            "DeclRefExpr" => format!("@({})", job.ref_decl.as_ref().unwrap().name,),
            _ => format!("{} [{}]", job.name, job.kind),
        };

        parent_connect
            .get_component("Label")
            .unwrap()
            .set("self", label);
    }

    let children = job
        .inner
        .iter()
        .filter(|i| {
            !matches!(
                i.kind.as_str(),
                "NamespaceDecl" | "UsingDirectiveDecl" | "TypedefDecl" | "CXXRecordDecl"
            )
        })
        .collect_vec();

    let child_count = children.len();
    let dx = 200.0 / layer;
    let old_pos = pos.clone();
    pos.y += 100.0;
    if child_count > 1 {
        pos.x -= (child_count as f32 / 2.0) * dx;
    }
    layer += 0.5;

    for child in children {
        if let Some(child_job) =
            ast_recursive_descent(window, child, Some(&parent_connect), pos, layer)
        {
            window.create_new_arrow(&parent_connect, &child_job, old_pos.lerp(pos, 0.5));
            pos.x += dx;
        }
    }

    if should_create {
        Some(parent_connect)
    } else {
        None
    }
}

pub fn cpp_importer(window: &mut GraspEditorWindow, content: String, path: PathBuf) {
    let cpp = path.canonicalize().unwrap().to_str().unwrap().to_string();

    let clang = Command::new("clang++")
        .args([
            "-Xclang",
            "-ast-dump=json",
            "-Xclang",
            "-ast-dump-filter=Foo",
            &cpp,
        ])
        .output()
        .expect("Failed to run clang");

    let json_raw = String::from_utf8_lossy(&clang.stdout).into_owned();

    let json_fixed = {
        let mut lines = VecDeque::from_iter(json_raw.lines());

        for line in &mut lines {
            if *line == "}" {
                *line = "},";
            }
        }

        lines.push_front("[");

        while let Some(line) = lines.pop_back() {
            if line.starts_with('}') {
                break;
            }
        }
        lines.push_back("}");

        lines.push_back("]");
        lines.iter().join("\n")
    };

    let _ = fs::write("tmp.json", json_fixed.as_bytes());
    let json: Vec<ASTNode> = serde_json::from_str(&json_fixed).unwrap();

    let mut x = 100.0f32;

    let file = json
        .clone()
        .into_iter()
        .find(|j| j.name == "Foo")
        .and_then(|n| n.loc.and_then(|n| n.file));

    for json in json
        .into_iter()
        .filter(|j| j.kind != "UsingShadowDecl")
        .filter(|j| j.loc.as_ref().and_then(|n| n.file.clone()) == file)
        .collect_vec()
    {
        let pos = Vec2::new(x, 200.0);
        let layer = 1.0f32;

        let _ = ast_recursive_descent(window, &json, None, pos, layer);
        x += 500.0;
    }
    //println!("{:?}", json);

    //let _ = fs::write("tmp.ast", format!("{:?}", json).as_bytes());
}
