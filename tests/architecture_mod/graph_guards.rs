use std::fs;
use std::path::Path;
use syn::{Attribute, Fields, Item, ItemStruct, Type};

/// 26. ГРАФОВАЯ АРХИТЕКТУРА (Relations Guard): Запрет на хранение Entity в компонентах.
/// Требует использования системы отношений Bevy 0.18.1.
#[test]
fn test_no_raw_entity_references_in_components() {
    check_entity_refs_recursive(Path::new("src"));
}

fn check_entity_refs_recursive(dir: &Path) {
    for entry in fs::read_dir(dir).expect("Could not read directory") {
        let entry = entry.expect("Invalid entry");
        let path = entry.path();

        if path.is_dir() {
            check_entity_refs_recursive(&path);
        } else if path.extension().map_or(false, |ext| ext == "rs") {
            let source = fs::read_to_string(&path).expect("Could not read Rust source file");
            let syntax = syn::parse_file(&source).expect("Could not parse Rust source file");
            for item in syntax.items {
                if let Item::Struct(item_struct) = item {
                    assert_component_has_no_raw_entity_refs(&path, &item_struct);
                }
            }
        }
    }
}

fn assert_component_has_no_raw_entity_refs(path: &Path, item_struct: &ItemStruct) {
    if !derives_component(&item_struct.attrs) || is_bevy_relationship(&item_struct.attrs) {
        return;
    }

    let has_raw_entity_reference = match &item_struct.fields {
        Fields::Named(fields) => fields
            .named
            .iter()
            .any(|field| type_contains_entity(&field.ty)),
        Fields::Unnamed(fields) => fields
            .unnamed
            .iter()
            .any(|field| type_contains_entity(&field.ty)),
        Fields::Unit => false,
    };
    assert!(
        !has_raw_entity_reference,
        "Graph Architecture Violation in {:?}: component '{}' stores Entity directly. Use a Bevy relationship instead.",
        path,
        item_struct.ident
    );
}

fn derives_component(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        let mut derives_component = false;
        if attribute.path().is_ident("derive") {
            let _ = attribute.parse_nested_meta(|meta| {
                if meta.path.is_ident("Component") {
                    derives_component = true;
                }
                Ok(())
            });
        }
        derives_component
    })
}

fn is_bevy_relationship(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        attribute.path().is_ident("relationship")
            || attribute.path().is_ident("relationship_target")
    })
}

fn type_contains_entity(ty: &Type) -> bool {
    let Type::Path(type_path) = ty else {
        return false;
    };
    let Some(segment) = type_path.path.segments.last() else {
        return false;
    };
    if segment.ident == "Entity" {
        return true;
    }
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return false;
    };
    arguments.args.iter().any(|argument| match argument {
        syn::GenericArgument::Type(inner_type) => type_contains_entity(inner_type),
        _ => false,
    })
}
