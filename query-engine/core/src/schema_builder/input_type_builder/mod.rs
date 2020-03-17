use super::*;

mod create_input_type_extension;
mod input_builder_extensions;
mod update_input_type_extension;

pub use create_input_type_extension::*;
pub use input_builder_extensions::*;
pub use update_input_type_extension::*;

pub trait InputTypeBuilderBase<'a>: CachedBuilder<InputObjectType> + InputBuilderExtensions {
    /// Builds scalar input fields using the mapper and the given, prefiltered, scalar fields.
    /// The mapper is responsible for mapping the fields to input types.
    fn scalar_input_fields<T, F>(
        &self,
        model_name: String,
        input_object_name: T,
        prefiltered_fields: Vec<ScalarFieldRef>,
        field_mapper: F,
        with_defaults: bool,
    ) -> Vec<InputField>
    where
        T: Into<String>,
        F: Fn(ScalarFieldRef) -> InputType,
    {
        let input_object_name = input_object_name.into();
        let mut non_list_fields: Vec<InputField> = prefiltered_fields
            .iter()
            .filter(|f| !f.is_list)
            .map(|f| {
                let default = if with_defaults {
                    f.default_value().cloned()
                } else {
                    None
                };
                input_field(f.name.clone(), field_mapper(Arc::clone(f)), default)
            })
            .collect();

        let mut list_fields: Vec<InputField> = prefiltered_fields
            .into_iter()
            .filter(|f| f.is_list)
            .map(|f| {
                let name = f.name.clone();
                let set_name = format!("{}{}{}Input", model_name, input_object_name, f.name);
                let input_object = match self.get_cache().get(&set_name) {
                    Some(t) => t,
                    None => {
                        let set_fields = vec![input_field("set", self.map_optional_input_type(&f), None)];
                        let input_object = Arc::new(input_object_type(set_name.clone(), set_fields));

                        self.cache(set_name, Arc::clone(&input_object));
                        Arc::downgrade(&input_object)
                    }
                };

                let set_input_type = InputType::opt(InputType::object(input_object));
                input_field(name, set_input_type, None)
            })
            .collect();

        non_list_fields.append(&mut list_fields);
        non_list_fields
    }

    /// Builds the "connect" input field for a relation.
    fn nested_connect_input_field(&self, field: RelationFieldRef) -> Option<InputField> {
        if field.related_model().is_embedded {
            None
        } else {
            Some(self.where_input_field("connect", field))
        }
    }

    fn where_input_field<T>(&self, name: T, field: RelationFieldRef) -> InputField
    where
        T: Into<String>,
    {
        let input_type = self.where_unique_object_type(&field.related_model());
        let input_type = Self::wrap_list_input_object_type(input_type, field.is_list);

        input_field(name.into(), input_type, None)
    }

    /// Wraps an input object type into an option list object type.
    fn wrap_list_input_object_type(input: InputObjectTypeRef, as_list: bool) -> InputType {
        if as_list {
            InputType::opt(InputType::list(InputType::object(input)))
        } else {
            InputType::opt(InputType::object(input))
        }
    }

    fn where_unique_object_type(&self, model: &ModelRef) -> InputObjectTypeRef {
        let name = format!("{}WhereUniqueInput", model.name);
        return_cached!(self.get_cache(), &name);

        let input_object = Arc::new(init_input_object_type(name.clone()));
        self.cache(name, Arc::clone(&input_object));

        // Single unique or ID fields.
        let unique_fields: Vec<ModelField> = model
            .fields()
            .all
            .iter()
            .filter(|f| f.is_unique())
            .filter_map(|f| {
                // We need to filter out m2m relations because they're never inlined on the model.
                // This is mostly a defensive precaution, as the parser should guarantee that the field
                // is inlined if it's unique or ID.
                // [DTODO] After checking the parser and tests, we might want to remove this filter.
                if let ModelField::Relation(rf) = f {
                    if rf.relation().is_many_to_many() {
                        None
                    } else {
                        Some(f)
                    }
                } else {
                    Some(f)
                }
            })
            .map(|f| f.clone())
            .collect();

        let mut fields: Vec<InputField> = unique_fields
            .into_iter()
            .map(|f| {
                let name = f.name().to_owned();

                let typ = match f {
                    ModelField::Scalar(ref sf) => self.map_optional_input_type(sf),
                    ModelField::Relation(ref rf) => InputType::opt(self.map_scalar_relation_input_type(rf)),
                };

                input_field(name, typ, None)
            })
            .collect();

        // @@unique compound fields.
        let compound_unique_fields: Vec<InputField> = model
            .unique_indexes()
            .into_iter()
            .map(|index| {
                let typ = self.compound_field_unique_object_type(index.name.as_ref(), index.fields());
                let name = compound_index_field_name(index);

                input_field(name, InputType::opt(InputType::object(typ)), None)
            })
            .collect();

        // @@id compound field (there can be only one per model).
        let id_fields = model.fields().id();
        let compound_id_field: Option<InputField> = if id_fields.as_ref().map(|f| f.len() > 1).unwrap_or(false) {
            id_fields.map(|fields| {
                let name = compound_id_field_name(&fields.iter().map(|f| f.name()).collect::<Vec<&str>>());
                let typ = self.compound_field_unique_object_type(None, fields);

                input_field(name, InputType::opt(InputType::object(typ)), None)
            })
        } else {
            None
        };

        fields.extend(compound_unique_fields);
        fields.extend(compound_id_field);

        input_object.set_fields(fields);

        Arc::downgrade(&input_object)
    }

    /// Generates and caches an input object type for a compound field.
    fn compound_field_unique_object_type(
        &self,
        alias: Option<&String>,
        from_fields: Vec<ModelField>,
    ) -> InputObjectTypeRef {
        let name = format!("{}CompoundUniqueInput", Self::compound_object_name(alias, &from_fields));
        return_cached!(self.get_cache(), &name);

        let input_object = Arc::new(init_input_object_type(name.clone()));
        self.cache(name, Arc::clone(&input_object));

        let object_fields = from_fields
            .into_iter()
            .map(|field| {
                let name = field.name().to_owned();

                let typ = match field {
                    ModelField::Scalar(ref sf) => self.map_required_input_type(sf),
                    ModelField::Relation(ref rf) => self.map_scalar_relation_input_type(rf),
                };

                input_field(name, typ, None)
            })
            .collect();

        input_object.set_fields(object_fields);
        Arc::downgrade(&input_object)
    }

    fn compound_object_name(alias: Option<&String>, from_fields: &[ModelField]) -> String {
        alias.map(|n| capitalize(n)).unwrap_or_else(|| {
            let field_names: Vec<String> = from_fields.iter().map(|field| capitalize(field.name())).collect();
            field_names.join("")
        })
    }

    /// Handles special cases where (non-m2m, inlined) relation fields can be used with scalar inputs.
    /// The input type is again a cached object type if the relation contains more than one data source field.
    fn map_scalar_relation_input_type(&self, relation_field: &RelationFieldRef) -> InputType {
        let dsfs = relation_field.data_source_fields();

        if dsfs.len() == 1 {
            return self.map_required_data_source_field_input_type(dsfs.first().unwrap());
        }

        let object_name = format!(
            "{}{}ScalarRelationInput",
            capitalize(&relation_field.model().name),
            capitalize(&relation_field.name)
        );

        if let Some(existing) = self.get_cache().get(&object_name) {
            return InputType::object(existing);
        };

        let input_object = Arc::new(init_input_object_type(object_name.clone()));
        self.cache(object_name, Arc::clone(&input_object));

        let object_fields = dsfs
            .into_iter()
            .map(|dsf| {
                let name = dsf.name.clone();
                let typ = self.map_required_data_source_field_input_type(&dsf);

                input_field(name, typ, None)
            })
            .collect();

        input_object.set_fields(object_fields);
        InputType::object(Arc::downgrade(&input_object))
    }

    fn get_filter_object_builder(&self) -> Arc<FilterObjectTypeBuilder<'a>>;
}

/// Central builder for input types.
/// The InputTypeBuilder differs in one major aspect from the original implementation:
/// It doesn't use options to represent if a type should be rendered or not.
/// Instead, empty input types (i.e. without fields) will be rendered and must be filtered on higher layers.
#[derive(Debug)]
pub struct InputTypeBuilder<'a> {
    internal_data_model: InternalDataModelRef,
    input_type_cache: TypeRefCache<InputObjectType>,
    filter_object_builder: Weak<FilterObjectTypeBuilder<'a>>,
}

impl<'a> CachedBuilder<InputObjectType> for InputTypeBuilder<'a> {
    fn get_cache(&self) -> &TypeRefCache<InputObjectType> {
        &self.input_type_cache
    }

    fn into_strong_refs(self) -> Vec<Arc<InputObjectType>> {
        self.input_type_cache.into()
    }
}

impl<'a> InputTypeBuilderBase<'a> for InputTypeBuilder<'a> {
    fn get_filter_object_builder(&self) -> Arc<FilterObjectTypeBuilder<'a>> {
        self.filter_object_builder
            .upgrade()
            .expect("Invariant violation: Expected input type builder reference to be valid")
    }
}

impl<'a> InputBuilderExtensions for InputTypeBuilder<'a> {}
impl<'a> CreateInputTypeBuilderExtension<'a> for InputTypeBuilder<'a> {}
impl<'a> UpdateInputTypeBuilderExtension<'a> for InputTypeBuilder<'a> {}

impl<'a> InputTypeBuilder<'a> {
    pub fn new(
        internal_data_model: InternalDataModelRef,
        filter_object_builder: Weak<FilterObjectTypeBuilder<'a>>,
    ) -> Self {
        InputTypeBuilder {
            internal_data_model,
            input_type_cache: TypeRefCache::new(),
            filter_object_builder,
        }
    }
}
