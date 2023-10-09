use clap::Args;
use clap_markdown::print_help_markdown;
use indexmap::IndexMap;
use schemars::schema_for;

// I was going to put this in a separate binary, but doing so would require that some of the code by pub instead of pub(crate). As I'm currently only considering commands and errors to be pub, that would be a problem, even if I'm not supporting a stable API.
use crate::Cosmopoeia;
use crate::commands::Task;
use crate::errors::CommandError;
use crate::subcommand_def;
use crate::progress::ProgressObserver;
use crate::world_map::LayerDocumentation;
use crate::world_map::document_tile_layer;
use crate::world_map::document_river_layer;
use crate::world_map::document_lake_layer;
use crate::world_map::document_biome_layer;
use crate::world_map::document_culture_layer;
use crate::world_map::document_town_layer;
use crate::world_map::document_nation_layer;
use crate::world_map::document_subnation_layer;
use crate::world_map::document_coastline_layer;
use crate::world_map::document_ocean_layer;
use crate::world_map::document_property_layer;
use crate::world_map::FieldTypeDocumentation;
use crate::commands::terrain::Command as TerrainCommand;
use crate::algorithms::culture_sets::CultureSetItemSource;
use crate::algorithms::naming::NamerSource;

fn list_schemas() -> Result<Vec<LayerDocumentation>,CommandError> {
    Ok(vec![
        document_tile_layer()?,
        document_biome_layer()?,
        document_coastline_layer()?,
        document_culture_layer()?,
        document_lake_layer()?,
        document_nation_layer()?,
        document_ocean_layer()?,
        document_property_layer()?,
        document_river_layer()?,
        document_subnation_layer()?,
        document_town_layer()?
    ])

}

fn map_field_types(field_type: &FieldTypeDocumentation, map: &mut IndexMap<String,FieldTypeDocumentation>) -> String {
    for sub_type in &field_type.sub_types {
        _ = map_field_types(sub_type, map)
    }
    let name = field_type.name.clone();
    match map.get(&name) {
        Some(existing) => if field_type != existing {
            // This is a programming logic error. FUTURE: I should find some type safe way to do this.
            panic!("Multiple documentation is listed for field type {}",name)
        },
        None => _ = map.insert(name.clone(), field_type.clone()),
    }
    name
}

fn write_world_file_schema_docs() -> Result<(),CommandError> {
    println!("# World File Schema");

    let mut formats = IndexMap::new();

    for schema in list_schemas()? {
        println!("## Layer `{}`",schema.name);
        println!("**geometry**: {}",schema.geometry);
        println!("");
        println!("{}",schema.description);
        println!("");
        for field in schema.fields {
            println!("### `{}`",field.name);
            println!("**database field type**: {}",map_field_types(&field.field_type,&mut formats));
            println!("");
            println!("{}",field.description);
            println!("");
        }
        println!()
    }

    formats.sort_keys();

    println!("## Field Types");

    for (name,field_type) in formats {
        println!("### {}",name);
        println!("**storage type**: {}",field_type.storage_type);
        println!("**syntax**: `{}`",field_type.syntax);
        println!("");
        println!("{}",field_type.description);
        println!("");
    }

    Ok(())
    
}

subcommand_def!{
    /// Writes documentation to a folder
    #[command(hide=true)]
    pub struct Docs {

    }
}

impl Task for Docs {
    fn run<Progress: ProgressObserver>(self, _: &mut Progress) -> Result<(),CommandError> {
        print_help_markdown::<Cosmopoeia>();
        write_world_file_schema_docs()?;
        write_terrain_task_schema_docs()?;
        write_culture_schema_docs()?;
        write_namer_schema_docs()?;
        Ok(())
    }
}

fn write_namer_schema_docs() -> Result<(),CommandError> {
    println!("{}",serde_json::to_string_pretty(&schema_for!(TerrainCommand)).map_err(|e| CommandError::TerrainProcessWrite(format!("Error writing schema for terrains: ({e}).")))?);
    Ok(())
}

fn write_culture_schema_docs() -> Result<(),CommandError> {
    println!("{}",serde_json::to_string_pretty(&schema_for!(CultureSetItemSource)).map_err(|e| CommandError::TerrainProcessWrite(format!("Error writing schema for terrains: ({e}).")))?);
    Ok(())
}

fn write_terrain_task_schema_docs() -> Result<(),CommandError> {
    println!("{}",serde_json::to_string_pretty(&schema_for!(NamerSource)).map_err(|e| CommandError::TerrainProcessWrite(format!("Error writing schema for terrains: ({e}).")))?);
    Ok(())
}
