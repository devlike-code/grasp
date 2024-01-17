// use std::sync::Arc;

// use itertools::Itertools;
// use mosaic::{
//     capabilities::{process::ProcessCapability, ArchetypeSubject, StringCapability, Traverse},
//     internals::{
//         all_tiles, leave_components, pars, ComponentValuesBuilderSetter, MosaicCollage, MosaicIO,
//         MosaicTypelevelCRUD, Tile, TileFieldEmptyQuery,
//     },
//     iterators::tile_getters::TileGetters,
// };

// use crate::utilities::Label;

// pub fn generate_enum(process_tile: &Tile) -> Tile {
//     let mosaic = Arc::clone(&process_tile.mosaic);
//     mosaic
//         .new_type("Error: { message: s128, target: u64, window: u64 };")
//         .unwrap();

//     if let Some((_, Some(enum_tile))) = mosaic
//         .get_process_parameter_values(process_tile)
//         .unwrap()
//         .iter()
//         .next()
//     {
//         match generate_enum_code(enum_tile) {
//             Ok(code) => mosaic.create_string_object(code.as_str()).unwrap(),
//             Err((str, target)) => mosaic.new_object(
//                 "Error",
//                 pars()
//                     .set("message", str.as_bytes())
//                     .set("target", target.id as u64)
//                     .ok(),
//             ),
//         }
//     } else {
//         mosaic.new_object(
//             "Error",
//             pars()
//                 .set(
//                     "message",
//                     "Cannot find enum tile - none passed as argument.",
//                 )
//                 .set("target", process_tile.id as u64)
//                 .ok(),
//         )
//     }
// }

// pub fn option_use_csharp_enum_naming_convention(enum_tile: &Tile) -> String {
//     if enum_tile
//         .get_component("CodeUseCSharpNamingConvention")
//         .is_some()
//     {
//         "E"
//     } else {
//         ""
//     }
//     .to_string()
// }

// pub fn option_indent_with_spaces(enum_tile: &Tile) -> String {
//     if enum_tile.get_component("CodeIndentWithSpaces").is_some() {
//         "  "
//     } else {
//         "\t"
//     }
//     .to_string()
// }

// pub fn generate_enum_code(enum_tile: &Tile) -> Result<String, (String, Tile)> {
//     let limit = enum_tile.mosaic.apply_collage(
//         &leave_components(
//             &["Process", "ProcessParameter", "ParameterBinding"],
//             all_tiles(),
//         ),
//         None,
//     );
//     let op = enum_tile
//         .mosaic
//         .traverse(mosaic::capabilities::Traversal::Limited {
//             tiles: limit.collect_vec(),
//             include_arrows: true,
//         });

//     let mut builder = "".to_string();

//     let spacing = option_indent_with_spaces(enum_tile);
//     let enum_naming = option_use_csharp_enum_naming_convention(enum_tile);

//     if let Some(name) = enum_tile.get_component("Label") {
//         builder += format!(
//             "internal enum {}{} {{\n",
//             enum_naming,
//             name.get("self").as_s32()
//         )
//         .as_str();

//         for member in op.get_arrows_into(enum_tile).get_sources() {
//             let member_name = Label(&member).query();
//             builder += format!("{}{},\n", spacing, member_name).as_str();
//         }
//         builder += "}\n";
//     }

//     Ok(builder)
// }

// #[cfg(test)]
// mod primitive_code_gen_tests {

//     use mosaic::{
//         capabilities::{process::ProcessCapability, StringCapability},
//         internals::{par, void, Mosaic, MosaicCRUD, MosaicIO, MosaicTypelevelCRUD},
//     };

//     use super::generate_enum;

//     #[test]
//     fn test_enums() {
//         let mosaic = Mosaic::new();
//         mosaic.new_type("Arrow: unit;").unwrap();
//         mosaic.new_type("Label: s32;").unwrap();
//         mosaic.new_type("Enum: s32;").unwrap();
//         mosaic.new_type("CodeIndentWithSpaces: unit;").unwrap();
//         mosaic
//             .new_type("CodeUseCSharpNamingConvention: unit;")
//             .unwrap();
//         let a = mosaic.new_object("Label", par("Variant"));
//         let b = mosaic.new_object("Label", par("Other"));
//         let c = mosaic.new_object("Label", par("Third"));
//         let e = mosaic.new_object("Label", par("MyEnum"));
//         mosaic.new_descriptor(&e, "Enum", par("MyEnum"));
//         for i in &[a, b, c] {
//             mosaic.new_arrow(&e, i, "Arrow", void());
//         }

//         mosaic.new_descriptor(&e, "CodeIndentWithSpaces", void());
//         mosaic.new_descriptor(&e, "CodeUseCSharpNamingConvention", void());

//         let p = mosaic.create_process("generate_enum", &["input"]).unwrap();
//         mosaic.pass_process_parameter(&p, "input", &e).unwrap();
//         println!("{}", mosaic.dot(""));

//         let r = generate_enum(&p);
//         assert!(r.component.is("String"));
//         println!("{}", mosaic.get_string_value(&r).unwrap());
//     }
// }
