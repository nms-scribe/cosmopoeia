use std::path::PathBuf;
use std::fs::File;
use std::io::Write;
use std::collections::HashMap;

use clap::Args;
use clap_markdown::help_markdown;
use indexmap::IndexMap;
use schemars::schema_for;
use schemars::schema::RootSchema;
use schemars::schema::SchemaObject;
use schemars::JsonSchema;
use schemars::schema::Metadata;
use schemars::schema::SingleOrVec;
use schemars::schema::InstanceType;
use schemars::schema::NumberValidation;
use schemars::schema::StringValidation;
use schemars::schema::ArrayValidation;
use schemars::schema::ObjectValidation;
use schemars::schema::SubschemaValidation;

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
use std::collections::BTreeSet;
use std::collections::BTreeMap;
use schemars::schema::Schema;

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

fn write_world_file_schema_docs(target: PathBuf) -> Result<(), CommandError> {
    let mut target = File::create(target)?;


    writeln!(&mut target,"# World File Schema")?;

    writeln!(&mut target)?;

    writeln!(&mut target, r#"
The world file output by Cosmpoeia is stored in a Geopackage (GPKG) file. This is a SQLite database that includes some pre-built tables for storing geographic information. It is best edited with GIS software that supports the format. Below is a description of the layers, or tables, contained inside the database, and field information. The field types given are internal to the software, and their database storage field type is defined in the Field Types scetion.

**On the FID field and Table Order**: Every layer in the file has an identifier field called `fid`, which contains a unique identifier for the field. This is handled by the gdal library, which Cosmopoeia uses for access to the file. Here are a few details:

* According to the [Geopackage standard](http://www.geopackage.org/spec131/index.html#feature_user_tables), the identifier field (which is called fid by default in gdal), is created with the following constraint in SQLite: `INTEGER PRIMARY KEY AUTOINCREMENT`.
* According to [SQLite documentation](https://www.sqlite.org/autoinc.html), a key defined in this way is guaranteed not to be reused, and appears to be possible to represent insertion order, as long as no parallel transactions are occurring, which I do not allow in the same instance of the program.
* According to tests, at least sometimes, when iterating through features, the features are returned from the database in fid order. I do not believe that this is guaranteed by any mechanism from gdal or sqlite.
* According to tests, a rust hashmap does not iterate over items in entry order. For this reason, I use a special map structure that iterates in fid order. This attempts to make it more likely that random operations with the same seed are always reproducible with the same input.

"#)?;

    let mut formats = IndexMap::new();

    for schema in list_schemas()? {
        writeln!(&mut target,"## Layer `{}`",schema.name)?;
        writeln!(&mut target,"**geometry**: {}",schema.geometry)?;
        writeln!(&mut target,"")?;
        writeln!(&mut target,"{}",schema.description)?;
        writeln!(&mut target,"")?;
        for field in schema.fields {
            writeln!(&mut target,"### `{}`",field.name)?;
            writeln!(&mut target,"**database field type**: {}",map_field_types(&field.field_type,&mut formats))?;
            writeln!(&mut target,"")?;
            writeln!(&mut target,"{}",field.description)?;
            writeln!(&mut target,"")?;
        }
        writeln!(&mut target,"")?;
    }

    formats.sort_keys();

    writeln!(&mut target,"## Field Types")?;

    for (name,field_type) in formats {
        writeln!(&mut target,"### {}",name)?;
        writeln!(&mut target,"**storage type**: {}",field_type.storage_type)?;
        writeln!(&mut target,"**syntax**: `{}`",field_type.syntax)?;
        writeln!(&mut target,"")?;
        writeln!(&mut target,"{}",field_type.description)?;
        writeln!(&mut target,"")?;
    }

    Ok(())
    
}


fn instance_type_name(instance_type: &InstanceType) -> &'static str {

    match instance_type {
        InstanceType::Null => "Null",
        InstanceType::Boolean => "Boolean",
        InstanceType::Object => "Object",
        InstanceType::Array => "Array",
        InstanceType::Number => "Number",
        InstanceType::String => "String",
        InstanceType::Integer => "Integer",
    }
    
}

fn join_iter<Str: AsRef<str>, Iter: Iterator<Item = Str>>(iter: Iter, delimiter: &str) -> String {
    iter.fold(String::new(),|mut a, b| {
        if !a.is_empty() {
            a.push_str(delimiter);
        }
        a.push_str(b.as_ref());
        a
    })
}

// The idea behind this struct is to catch major changes to the json schema in compilation.
// properties not listed here are not supported, and will cause an error.
struct UsableSchema {
    any_of: Option<Vec<UsableSchema>>,
    one_of: Option<Vec<UsableSchema>>,
    format: Option<String>,
    enum_values: Option<Vec<serde_json::Value>>,
    title: Option<String>,
    description: Option<String>,
    instance_types: Option<Vec<InstanceType>>,
    reference: Option<String>,
    minimum: Option<f64>,
    min_length: Option<u32>,
    max_length: Option<u32>,
    pattern: Option<String>,
    min_items: Option<u32>,
    max_items: Option<u32>,
    items: Option<Vec<UsableSchema>>,
    required_props: BTreeSet<String>,
    properties: Option<BTreeMap<String, UsableSchema>>,
    additional_properties: Option<Box<UsableSchemaOrBoolean>>
}

enum UsableSchemaOrBoolean {
    Boolean(bool),
    Schema(UsableSchema)
}

impl UsableSchemaOrBoolean {

    fn from(schema: Schema) -> Self {
        match schema {
            Schema::Bool(value) => Self::Boolean(value),
            Schema::Object(schema) => Self::Schema(UsableSchema::from(schema))
        }
    }
}

impl UsableSchema {

    fn from(schema: SchemaObject) -> Self {

        let SchemaObject {
            metadata,
            instance_type,
            format,
            enum_values,
            const_value,
            subschemas,
            number,
            string,
            array,
            object,
            reference,
            extensions,
        } = schema;
    
        if !extensions.is_empty() {
            unimplemented!("Extensions are not supported yet.")
        }
    
        if let Some(_) = const_value {
            unimplemented!("const_value isn't supported yet.");
        }

        let reference = if let Some(reference) = reference {
            if let Some(reference) = reference.strip_prefix("#/definitions/") {
                Some(reference.to_owned())
            } else {
                unimplemented!("reference did not have the expected prefix.")
            }
        } else {
            None
        };
    
        // metadata
        let (title,description,default) = if let Some(metadata) = metadata {
            let Metadata{ title, description, default, .. } = *metadata;
            (title,description,default)
        } else {
            (None,None,None)
        };
    
        // default value
        if let Some(_) = default {
            unimplemented!("default value isn't supported yet.");
        }
    
        let instance_types = if let Some(instance_type) = &instance_type {
            match instance_type {
                SingleOrVec::Single(instance_type) => Some(vec![**instance_type]),
                SingleOrVec::Vec(vec) => Some(vec.into_iter().map(|t| *t).collect()),
            }
        } else {
            None
        };
    
        let minimum = if let Some(number) = number {
            let NumberValidation{exclusive_minimum,minimum,maximum,exclusive_maximum,multiple_of} = *number;
            if exclusive_minimum.is_some() {
                unimplemented!("exclusive minimum isn't supported yet.")
            }
            if exclusive_maximum.is_some() {
                unimplemented!("exclusive maximum isn't supported yet.")
            }
            if maximum.is_some() {
                unimplemented!("maximum isn't supported yet.")
            }
            if multiple_of.is_some() {
                unimplemented!("multiple_of isn't supported yet.")
            }
            minimum
        } else {
            None
        };
    
        let (min_length,max_length,pattern) = if let Some(string) = string {
            let StringValidation{min_length,max_length,pattern} = *string;
            (min_length,max_length,pattern)
        } else {
            (None,None,None)
        };
    
        let (min_items,max_items,items) = if let Some(array) = array {
            let ArrayValidation{min_items,max_items,unique_items,items,additional_items,contains} = *array;
            if additional_items.is_some() {
                unimplemented!("additional items isn't supported yet.")
            }
            if unique_items.is_some() {
                unimplemented!("unique items isn't supported yet.")
            }
            if contains.is_some() {
                unimplemented!("contains isn't supported yet.")
            }
            let items = if let Some(items) = items {
                match items {
                    SingleOrVec::Single(schema) => match *schema {
                        Schema::Bool(_) => unimplemented!("boolean schemas for single item isn't supported yet"),
                        Schema::Object(schema) => Some(vec![UsableSchema::from(schema)]),
                    },
                    SingleOrVec::Vec(schemas) => Some(schemas.into_iter().map(|s| {
                        match s {
                            Schema::Bool(_) => unimplemented!("boolean schemas for items isn't supported yet"),
                            Schema::Object(schema) => UsableSchema::from(schema),
                        }
                    }).collect()),
                }
            } else {
                None
            };
            (min_items,max_items,items)
        } else {
            (None,None,None)
        };
    
        let (required_props, properties, additional_properties) =  if let Some(object) = object {
            let ObjectValidation{properties, additional_properties, max_properties, min_properties, required, pattern_properties, property_names } = *object;
            if max_properties.is_some() {
                unimplemented!("max_properties isn't supported yet.")
            }
            if min_properties.is_some() {
                unimplemented!("min_properties isn't supported yet.")
            }
            if !pattern_properties.is_empty() {
                unimplemented!("pattern_properties isn't supported yet.")
            }
            if property_names.is_some() {
                unimplemented!("property_names isn't supported yet.")
            }
            let properties = if properties.is_empty() {
                None
            } else {
                let properties = BTreeMap::from_iter(properties.into_iter().map(|(k,v)| {
                    let v = match v {
                        Schema::Bool(_) => unimplemented!("Boolean schemas for properties isn't supported yet."),
                        Schema::Object(schema) => UsableSchema::from(schema),
                    };
                    (k,v)
                }));
                Some(properties)
            };
            let additional_properties = if let Some(additional_properties) = additional_properties {
                Some(UsableSchemaOrBoolean::from(*additional_properties).into())
            } else {
                None
            };
            (required, properties, additional_properties)
        } else {
            (Default::default(),None,None)
        };

        let (any_of,one_of) = if let Some(subschemas) = subschemas {
            let SubschemaValidation{any_of,one_of,not, all_of, if_schema, then_schema, else_schema } = *subschemas;
            if all_of.is_some() {
                unimplemented!("all_of isn't supported yet.")
            }
            if if_schema.is_some() {
                unimplemented!("if_schema isn't supported yet.")
            }
            if then_schema.is_some() {
                unimplemented!("then_schema isn't supported yet.")
            }
            if else_schema.is_some() {
                unimplemented!("else_schema isn't supported yet.")
            }
            if not.is_some() {
                unimplemented!("not isn't supported yet.")
            }
            let any_of = if let Some(any_of) = any_of {
                Some(any_of.into_iter().map(|s|{
                    match s {
                        Schema::Bool(_) => unimplemented!("boolean schemas in any_of are not supported yet."),
                        Schema::Object(s) => UsableSchema::from(s),
                    }
                }).collect())
            } else {
                None
            };
            let one_of = if let Some(one_of) = one_of {
                Some(one_of.into_iter().map(|s|{
                    match s {
                        Schema::Bool(_) => unimplemented!("boolean schemas in one_of are not supported yet."),
                        Schema::Object(s) => UsableSchema::from(s),
                    }
                }).collect())
            } else {
                None
            };
            (any_of,one_of)
        } else {
            (None,None)
        };

        Self{
            title,
            description,
            instance_types,        
            reference,
            format,
            enum_values,
            minimum,
            min_length,
            max_length,
            pattern,
            min_items,
            max_items,
            items,
            required_props,
            properties,
            additional_properties,
            any_of,
            one_of

        }
    }

}



const TAB: &str = "  ";

fn write_root_schema(default_title: String, root: RootSchema, target: &mut File) -> Result<(),CommandError> {

    // Output format inspired by https://pypi.org/project/jsonschema2md/

    let RootSchema{
        schema,
        definitions,
        meta_schema
    } = root;

    match meta_schema {
        // this was the value being passed at the time I was developing this.
        Some(meta) if meta == "http://json-schema.org/draft-07/schema#" => (),
        Some(meta) => unimplemented!("meta_schema '{meta}' is not supported."),
        None => ()
    }

    let schema = UsableSchema::from(schema);

    if let Some(title) = schema.title {
        writeln!(target,"# {title}")?;
    } else {
        writeln!(target,"# {default_title}")?;
    }
    writeln!(target)?;

    if let Some(description) = schema.description {
        writeln!(target,"{description}")?;
        writeln!(target)?;
    }

    if let Some(items) = schema.items {
        writeln!(target,"## Items")?;
        writeln!(target)?;
        writeln!(target,"{TAB}* **Items**:")?;

        for schema in items {
            write_schema(schema, None, None, false, 2, target)?
        }
    }

    if let Some(properties) = schema.properties {
        writeln!(target,"## Properties")?;
        for (name,property_schema) in properties {
            let is_required = schema.required_props.contains(&name);
            write_schema(property_schema, Some((name,true)), None, is_required, 1, target)?;
        }
    }

    // FUTURE: If we support patternProperties, then output each under header 'Pattern Properties'

    if let Some(UsableSchemaOrBoolean::Schema(additional_properties)) = schema.additional_properties.map(|s| *s) {
        writeln!(target,"## Additional Properties")?;
        writeln!(target)?;
        write_schema(additional_properties, Some(("Additional Properties".to_owned(),false)), None, false, 0, target)?;
    }

    if !definitions.is_empty() {
        writeln!(target,"## Definitions")?;
        for (name,schema) in definitions {
            match schema {
                Schema::Bool(_) => unimplemented!("Boolean schemas in definitions are not yet supported."),
                Schema::Object(schema) => {
                    let schema = UsableSchema::from(schema);
                    let anchor = format!("definitions/{name}");
                    write_schema(schema, Some((name,true)), Some(anchor), false, 1, target)?;
                },
            }
        }

    }

    Ok(())

}


fn write_schema(schema: UsableSchema, name: Option<(String,bool)> /* name, code_span? */, anchor: Option<String>, required: bool, level: usize, target: &mut File) -> Result<(),CommandError> {

    let indent = TAB.repeat(level);

    write!(target,"{indent}* ")?;

    let mut has_term = false;

    if let Some(anchor) = anchor {
        has_term = true;
        write!(target,"<a id=\"{anchor}\"></a>")?
    };

    if let Some((name,code_span)) = name {
        has_term = true;
        if code_span {
            write!(target,"**`{name}`**")?;
        } else {
            write!(target,"**{name}**")?;
        }
    };


    if schema.instance_types.is_some() || schema.reference.is_some() || schema.format.is_some() || required {
        let mut spacing = if has_term {
            " "
        } else {
            ""
        };

        has_term = true;

        write!(target,"{spacing}*(")?;
        spacing = "";

        let mut has_type = false;

        if let Some(instance_types) = &schema.instance_types {
            has_type = true;
            let instance_types = join_iter(instance_types.into_iter().map(|i| instance_type_name(&i))," | ");
            write!(target,"{instance_types}")?;
            spacing = ", ";
        };

        if let Some(reference) = &schema.reference {
            if has_type {
                spacing = " | "
            };
            write!(target,"{spacing}[{reference}](#definitions/{reference})")?;
            spacing = ", ";
        }

        if let Some(format) = &schema.format {
            write!(target,"{spacing}Format: {format}")?;
            spacing = ", ";
        }

        if required {
            write!(target,"{spacing}Required")?;
        }

        write!(target,")*")?;


    }

    

    write_description(&schema, has_term, target)?;

    // FUTURE: Support all_of in the same way as any_of

    if let Some(any_of) = schema.any_of {
        writeln!(target,"{indent}{TAB}* **Any of**")?;
        for schema in any_of {
            write_schema(schema, None, None, false, level + 2, target)?;
        }
    }

    if let Some(one_of) = schema.one_of {
        writeln!(target,"{indent}{TAB}* **One of**")?;
        for schema in one_of {
            write_schema(schema, None, None, false, level + 2, target)?;
        }
    }

    if let Some(items) = schema.items {

        writeln!(target,"{indent}{TAB}* **Items**:")?;

        for schema in items {
            write_schema(schema, None, None, false, level + 2, target)?
        }
    }

    if let Some(UsableSchemaOrBoolean::Schema(additional_properties)) = schema.additional_properties.map(|s| *s) {
        write_schema(additional_properties, Some(("Additional Properties".to_owned(),false)), None, false, level + 1, target)?;
    }

    if let Some(properties) = schema.properties {
        for (name,property_schema) in properties {
            let is_required = schema.required_props.contains(&name);
            write_schema(property_schema, Some((name,true)), None, is_required, level + 1, target)?;
        }
    }

    // FUTURE: If we support patternProperties, then output each under header 'Pattern Properties'

    Ok(())

}

fn write_description(schema: &UsableSchema, has_term: bool, target: &mut File) -> Result<(), CommandError> {

    let mut spacing = if has_term {
        ": "
    } else {
        ""
    };

    if let Some(description) = &schema.description {
        write!(target,"{spacing}{description}")?;
        spacing = " ";
    }

    if let Some(minimum) = schema.minimum {
        write!(target,"{spacing}Minimum: `{minimum}`")?;
        spacing = ", ";
    }

    if let Some(pattern) = &schema.pattern {
        write!(target,"{spacing}Pattern: `{pattern}`")?;
        spacing = ", ";
    }

    if let Some(min_length) = &schema.min_length {
        write!(target,"{spacing}Minimum Length: `{min_length}`")?;
        spacing = ", ";
    }

    if let Some(max_length) = &schema.max_length {
        write!(target,"{spacing}Maximum Length: `{max_length}`")?;
        spacing = ", ";
    }

    if let Some(min_items) = &schema.min_items {
        write!(target,"{spacing}Minimum Items: `{min_items}`")?;
        spacing = ", ";
    }

    if let Some(max_items) = &schema.max_items {
        write!(target,"{spacing}Maximum Items: `{max_items}`")?;
        spacing = ", ";
    }

    if let Some(enum_values) = &schema.enum_values {
        if enum_values.len() == 1 {
            let enum_values = serde_json::to_string(&enum_values[0])?;
            write!(target,"{spacing}Must be: {enum_values}")?;
        } else {
            let enum_values = serde_json::to_string(&enum_values)?;
            write!(target,"{spacing}Must be one of: {enum_values}")?;
        }
        spacing = ", ";
    }

    if let Some(additional_properties) = &schema.additional_properties {
        match **additional_properties {
            UsableSchemaOrBoolean::Boolean(false) => write!(target,"{spacing}Can not contain additional properties.")?,
            UsableSchemaOrBoolean::Boolean(true) |
            UsableSchemaOrBoolean::Schema(_) => write!(target,"{spacing}Can contain additional properties.")?,
        }
    }

    writeln!(target)?;

    Ok(())
}


fn write_command_help(target: PathBuf) -> Result<(),CommandError> {
    let mut target = File::create(target)?;
    write!(&mut target,"{}",help_markdown::<Cosmopoeia>())?;
    Ok(())
}

fn write_schema_docs<Schema: JsonSchema>(title: String, schema_target: PathBuf, docs_target: PathBuf) -> Result<(),CommandError> {
    let mut schema_target = File::create(schema_target)?;
    let schema = schema_for!(Schema);
    write!(&mut schema_target,"{}",serde_json::to_string_pretty(&schema)?)?;
    write_root_schema(title,schema,&mut File::create(docs_target)?)
}


subcommand_def!{
    /// Writes generatable documentation and json schemas to a folder. This is incomplete as it does not generate docs for the json schemas.
    #[command(hide=true)]
    pub struct Docs {

        #[arg(long)]
        /// The folder to output the generated documentation to
        docs: PathBuf,

        #[arg(long)]
        /// The folder to output generated schemas to
        schemas: PathBuf


    }
}

impl Task for Docs {
    fn run<Progress: ProgressObserver>(self, _: &mut Progress) -> Result<(),CommandError> {
        let command_help = self.docs.join("Commands.md");
        write_command_help(command_help)?;
        let world_file_schema = self.docs.join("World File Schema.md");
        write_world_file_schema_docs(world_file_schema)?;
        
        let terrain_task_schema = self.schemas.join("terrain_tasks.schema.json");
        let terrain_task_doc = self.docs.join("Recipe Set Schema.md");
        write_schema_docs::<HashMap<String,Vec<TerrainCommand>>>("Terrain Recipe Set".to_owned(),terrain_task_schema,terrain_task_doc)?;
        
        let culture_set_schema = self.schemas.join("cultures.schema.json");
        let culture_set_doc = self.docs.join("Cultures Schema.md");
        write_schema_docs::<Vec<CultureSetItemSource>>("Culture Set".to_owned(),culture_set_schema,culture_set_doc)?;

        let namer_schema = self.schemas.join("namers.schema.json");
        let namer_docs = self.docs.join("Namers Schema.md");
        write_schema_docs::<Vec<NamerSource>>("Namer Set".to_owned(),namer_schema,namer_docs)?;
        Ok(())

        /*
        
        FUTURE: Getting markdown from the schemas, I've tried
        - https://github.com/adobe/jsonschema2md
           resulted in: TypeError [ERR_INVALID_ARG_TYPE]: The "path" argument must be of type string. Received undefined
           research: unfixed bug described at https://github.com/adobe/jsonschema2md/issues/392
        - https://github.com/GefenOnline/JSON-schema-to-markdown
           resulted in documents with lots of missing information.
           research: statement in readme: "This module does not implement anywhere near the full RFC specs."
        - https://github.com/BrianWendt/json-schema-md-doc
           also resulted in document with missing information, but not as much
           reasearch: code is at least 3 years old, perhaps there's something wrong.
         */
    }
}

