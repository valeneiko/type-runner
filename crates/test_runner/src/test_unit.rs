use core::str;
use std::{iter, path::Path};

use compact_str::CompactString;
use memchr::{memchr, memchr_iter};
use oxc_index::IndexVec;
use rustc_hash::FxHashMap;

use crate::byte_utils::{trim_space, trim_space_end, trim_space_start};

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct TestSettings {
    pub no_types_and_symbols: bool,
    pub base_url: Option<CompactString>,
    pub no_implicit_references: bool,
    pub include_built_file: Option<CompactString>,
    pub lib_files: Option<Vec<CompactString>>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum TestVariationProp {
    AllowArbitraryExtensions,
    AllowImportingTsExtensions,
    AllowJS,
    ESModuleInterop,
    ExactOptionalPropertyTypes,
    IsolatedModules,
    Jsx,
    Module,
    ModuleDetection,
    ModuleResolution,
    NoEmit,
    NoImplicitAny,
    NoImplicitOverride,
    NoPropertyAccessFromIndexSignature,
    NoUncheckedIndexedAccess,
    NoUncheckedSideEffectImports,
    PreserveConstEnums,
    ResolveJsonModule,
    ResolvePackageJsonExports,
    Strict,
    StrictBuiltinIteratorReturn,
    StrictNullChecks,
    Target,
    UseDefineForClassFields,
    UseUnknownInCatchVariables,
    VerbatimModuleSyntax,
}

const TEST_VARIATION_PROPS: &[TestVariationProp] = &[
    TestVariationProp::AllowArbitraryExtensions,
    TestVariationProp::AllowImportingTsExtensions,
    TestVariationProp::AllowJS,
    TestVariationProp::ESModuleInterop,
    TestVariationProp::ExactOptionalPropertyTypes,
    TestVariationProp::IsolatedModules,
    TestVariationProp::Jsx,
    TestVariationProp::Module,
    TestVariationProp::ModuleDetection,
    TestVariationProp::ModuleResolution,
    TestVariationProp::NoEmit,
    TestVariationProp::NoImplicitAny,
    TestVariationProp::NoImplicitOverride,
    TestVariationProp::NoPropertyAccessFromIndexSignature,
    TestVariationProp::NoUncheckedIndexedAccess,
    TestVariationProp::NoUncheckedSideEffectImports,
    TestVariationProp::PreserveConstEnums,
    TestVariationProp::ResolveJsonModule,
    TestVariationProp::ResolvePackageJsonExports,
    TestVariationProp::Strict,
    TestVariationProp::StrictBuiltinIteratorReturn,
    TestVariationProp::StrictNullChecks,
    TestVariationProp::Target,
    TestVariationProp::UseDefineForClassFields,
    TestVariationProp::UseUnknownInCatchVariables,
    TestVariationProp::VerbatimModuleSyntax,
];

impl TestVariationProp {
    fn expand_wildcard(self) -> Vec<CompactString> {
        match self {
            TestVariationProp::Module => vec![
                "amd".into(),
                "es6".into(),
                "umd".into(),
                "none".into(),
                "es2020".into(),
                "es2022".into(),
                "esnext".into(),
                "node16".into(),
                "node18".into(),
                "system".into(),
                "commonjs".into(),
                "nodenext".into(),
                "preserve".into(),
            ],
            TestVariationProp::StrictBuiltinIteratorReturn
            | TestVariationProp::UseDefineForClassFields
            | TestVariationProp::Strict => vec!["true".into(), "false".into()],
            _ => panic!("Wildcard not defined for: {self:?}"),
        }
    }
}

impl From<TestVariationProp> for &str {
    fn from(value: TestVariationProp) -> Self {
        match value {
            TestVariationProp::AllowArbitraryExtensions => "allowarbitraryextensions",
            TestVariationProp::AllowImportingTsExtensions => "allowimportingtsextensions",
            TestVariationProp::AllowJS => "allowjs",
            TestVariationProp::ESModuleInterop => "esmoduleinterop",
            TestVariationProp::ExactOptionalPropertyTypes => "exactoptionalpropertytypes",
            TestVariationProp::IsolatedModules => "isolatedmodules",
            TestVariationProp::Jsx => "jsx",
            TestVariationProp::Module => "module",
            TestVariationProp::ModuleDetection => "moduledetection",
            TestVariationProp::ModuleResolution => "moduleresolution",
            TestVariationProp::NoEmit => "noemit",
            TestVariationProp::NoImplicitAny => "noimplicitany",
            TestVariationProp::NoImplicitOverride => "noimplicitoverride",
            TestVariationProp::NoPropertyAccessFromIndexSignature => {
                "nopropertyaccessfromindexsignature"
            }
            TestVariationProp::NoUncheckedIndexedAccess => "nouncheckedindexedaccess",
            TestVariationProp::NoUncheckedSideEffectImports => "nouncheckedsideeffectimports",
            TestVariationProp::PreserveConstEnums => "preserveconstenums",
            TestVariationProp::ResolveJsonModule => "resolvejsonmodule",
            TestVariationProp::ResolvePackageJsonExports => "resolvepackagejsonexports",
            TestVariationProp::Strict => "strict",
            TestVariationProp::StrictBuiltinIteratorReturn => "strictbuiltiniteratorreturn",
            TestVariationProp::StrictNullChecks => "strictnullchecks",
            TestVariationProp::Target => "target",
            TestVariationProp::UseDefineForClassFields => "usedefineforclassfields",
            TestVariationProp::UseUnknownInCatchVariables => "useunknownincatchvariables",
            TestVariationProp::VerbatimModuleSyntax => "verbatimmodulesyntax",
        }
    }
}

impl TryFrom<&[u8]> for TestVariationProp {
    type Error = ();

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        match value {
            b"allowarbitraryextensions" => Ok(TestVariationProp::AllowArbitraryExtensions),
            b"allowimportingtsextensions" => Ok(TestVariationProp::AllowImportingTsExtensions),
            b"allowjs" => Ok(TestVariationProp::AllowJS),
            b"esmoduleinterop" => Ok(TestVariationProp::ESModuleInterop),
            b"exactoptionalpropertytypes" => Ok(TestVariationProp::ExactOptionalPropertyTypes),
            b"isolatedmodules" => Ok(TestVariationProp::IsolatedModules),
            b"jsx" => Ok(TestVariationProp::Jsx),
            b"module" => Ok(TestVariationProp::Module),
            b"moduledetection" => Ok(TestVariationProp::ModuleDetection),
            b"moduleresolution" => Ok(TestVariationProp::ModuleResolution),
            b"noemit" => Ok(TestVariationProp::NoEmit),
            b"noimplicitany" => Ok(TestVariationProp::NoImplicitAny),
            b"noimplicitoverride" => Ok(TestVariationProp::NoImplicitOverride),
            b"nopropertyaccessfromindexsignature" => {
                Ok(TestVariationProp::NoPropertyAccessFromIndexSignature)
            }
            b"nouncheckedindexedaccess" => Ok(TestVariationProp::NoUncheckedIndexedAccess),
            b"nouncheckedsideeffectimports" => Ok(TestVariationProp::NoUncheckedSideEffectImports),
            b"preserveconstenums" => Ok(TestVariationProp::PreserveConstEnums),
            b"resolvejsonmodule" => Ok(TestVariationProp::ResolveJsonModule),
            b"resolvepackagejsonexports" => Ok(TestVariationProp::ResolvePackageJsonExports),
            b"strict" => Ok(TestVariationProp::Strict),
            b"strictbuiltiniteratorreturn" => Ok(TestVariationProp::StrictBuiltinIteratorReturn),
            b"strictnullchecks" => Ok(TestVariationProp::StrictNullChecks),
            b"target" => Ok(TestVariationProp::Target),
            b"usedefineforclassfields" => Ok(TestVariationProp::UseDefineForClassFields),
            b"useunknownincatchvariables" => Ok(TestVariationProp::UseUnknownInCatchVariables),
            b"verbatimmodulesyntax" => Ok(TestVariationProp::VerbatimModuleSyntax),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct TestVariations {
    pub allow_arbitrary_extensions: Vec<CompactString>,
    pub allow_importing_ts_extensions: Vec<CompactString>,
    pub allow_js: Vec<CompactString>,
    pub es_module_interop: Vec<CompactString>,
    pub exact_optional_property_types: Vec<CompactString>,
    pub isolated_modules: Vec<CompactString>,
    pub jsx: Vec<CompactString>,
    pub module: Vec<CompactString>,
    pub module_detection: Vec<CompactString>,
    pub module_resolution: Vec<CompactString>,
    pub no_emit: Vec<CompactString>,
    pub no_implicit_any: Vec<CompactString>,
    pub no_implicit_override: Vec<CompactString>,
    pub no_property_access_from_index_signature: Vec<CompactString>,
    pub no_unchecked_indexed_access: Vec<CompactString>,
    pub no_unchecked_side_effect_imports: Vec<CompactString>,
    pub preserve_const_enums: Vec<CompactString>,
    pub resolve_json_module: Vec<CompactString>,
    pub resolve_package_json_exports: Vec<CompactString>,
    pub strict: Vec<CompactString>,
    pub strict_builtin_iterator_return: Vec<CompactString>,
    pub strict_null_checks: Vec<CompactString>,
    pub target: Vec<CompactString>,
    pub use_define_for_class_fields: Vec<CompactString>,
    pub use_unknown_in_catch_variables: Vec<CompactString>,
    pub verbatim_module_syntax: Vec<CompactString>,
}

impl TestVariations {
    fn get(&self, prop: TestVariationProp) -> &Vec<CompactString> {
        match prop {
            TestVariationProp::AllowArbitraryExtensions => &self.allow_arbitrary_extensions,
            TestVariationProp::AllowImportingTsExtensions => &self.allow_importing_ts_extensions,
            TestVariationProp::AllowJS => &self.allow_js,
            TestVariationProp::ESModuleInterop => &self.es_module_interop,
            TestVariationProp::ExactOptionalPropertyTypes => &self.exact_optional_property_types,
            TestVariationProp::IsolatedModules => &self.isolated_modules,
            TestVariationProp::Jsx => &self.jsx,
            TestVariationProp::Module => &self.module,
            TestVariationProp::ModuleDetection => &self.module_detection,
            TestVariationProp::ModuleResolution => &self.module_resolution,
            TestVariationProp::NoEmit => &self.no_emit,
            TestVariationProp::NoImplicitAny => &self.no_implicit_any,
            TestVariationProp::NoImplicitOverride => &self.no_implicit_override,
            TestVariationProp::NoPropertyAccessFromIndexSignature => {
                &self.no_property_access_from_index_signature
            }
            TestVariationProp::NoUncheckedIndexedAccess => &self.no_unchecked_indexed_access,
            TestVariationProp::NoUncheckedSideEffectImports => {
                &self.no_unchecked_side_effect_imports
            }
            TestVariationProp::PreserveConstEnums => &self.preserve_const_enums,
            TestVariationProp::ResolveJsonModule => &self.resolve_json_module,
            TestVariationProp::ResolvePackageJsonExports => &self.resolve_package_json_exports,
            TestVariationProp::Strict => &self.strict,
            TestVariationProp::StrictBuiltinIteratorReturn => &self.strict_builtin_iterator_return,
            TestVariationProp::StrictNullChecks => &self.strict_null_checks,
            TestVariationProp::Target => &self.target,
            TestVariationProp::UseDefineForClassFields => &self.use_define_for_class_fields,
            TestVariationProp::UseUnknownInCatchVariables => &self.use_unknown_in_catch_variables,
            TestVariationProp::VerbatimModuleSyntax => &self.verbatim_module_syntax,
        }
    }

    fn push(&mut self, prop: TestVariationProp, value: CompactString) {
        match prop {
            TestVariationProp::AllowArbitraryExtensions => {
                self.allow_arbitrary_extensions.push(value);
            }
            TestVariationProp::AllowImportingTsExtensions => {
                self.allow_importing_ts_extensions.push(value);
            }
            TestVariationProp::AllowJS => self.allow_js.push(value),
            TestVariationProp::ESModuleInterop => self.es_module_interop.push(value),
            TestVariationProp::ExactOptionalPropertyTypes => {
                self.exact_optional_property_types.push(value);
            }
            TestVariationProp::IsolatedModules => self.isolated_modules.push(value),
            TestVariationProp::Jsx => self.jsx.push(value),
            TestVariationProp::Module => self.module.push(value),
            TestVariationProp::ModuleDetection => self.module_detection.push(value),
            TestVariationProp::ModuleResolution => self.module_resolution.push(value),
            TestVariationProp::NoEmit => self.no_emit.push(value),
            TestVariationProp::NoImplicitAny => self.no_implicit_any.push(value),
            TestVariationProp::NoImplicitOverride => self.no_implicit_override.push(value),
            TestVariationProp::NoPropertyAccessFromIndexSignature => {
                self.no_property_access_from_index_signature.push(value);
            }
            TestVariationProp::NoUncheckedIndexedAccess => {
                self.no_unchecked_indexed_access.push(value);
            }
            TestVariationProp::NoUncheckedSideEffectImports => {
                self.no_unchecked_side_effect_imports.push(value);
            }
            TestVariationProp::PreserveConstEnums => self.preserve_const_enums.push(value),
            TestVariationProp::ResolveJsonModule => self.resolve_json_module.push(value),
            TestVariationProp::ResolvePackageJsonExports => {
                self.resolve_package_json_exports.push(value);
            }
            TestVariationProp::Strict => self.strict.push(value),
            TestVariationProp::StrictBuiltinIteratorReturn => {
                self.strict_builtin_iterator_return.push(value);
            }
            TestVariationProp::StrictNullChecks => self.strict_null_checks.push(value),
            TestVariationProp::Target => self.target.push(value),
            TestVariationProp::UseDefineForClassFields => {
                self.use_define_for_class_fields.push(value);
            }
            TestVariationProp::UseUnknownInCatchVariables => {
                self.use_unknown_in_catch_variables.push(value);
            }
            TestVariationProp::VerbatimModuleSyntax => self.verbatim_module_syntax.push(value),
        }
    }

    fn clear(&mut self, prop: TestVariationProp) {
        match prop {
            TestVariationProp::AllowArbitraryExtensions => self.allow_arbitrary_extensions.clear(),
            TestVariationProp::AllowImportingTsExtensions => {
                self.allow_importing_ts_extensions.clear();
            }
            TestVariationProp::AllowJS => self.allow_js.clear(),
            TestVariationProp::ESModuleInterop => self.es_module_interop.clear(),
            TestVariationProp::ExactOptionalPropertyTypes => {
                self.exact_optional_property_types.clear();
            }
            TestVariationProp::IsolatedModules => self.isolated_modules.clear(),
            TestVariationProp::Jsx => self.jsx.clear(),
            TestVariationProp::Module => self.module.clear(),
            TestVariationProp::ModuleDetection => self.module_detection.clear(),
            TestVariationProp::ModuleResolution => self.module_resolution.clear(),
            TestVariationProp::NoEmit => self.no_emit.clear(),
            TestVariationProp::NoImplicitAny => self.no_implicit_any.clear(),
            TestVariationProp::NoImplicitOverride => self.no_implicit_override.clear(),
            TestVariationProp::NoPropertyAccessFromIndexSignature => {
                self.no_property_access_from_index_signature.clear();
            }
            TestVariationProp::NoUncheckedIndexedAccess => self.no_unchecked_indexed_access.clear(),
            TestVariationProp::NoUncheckedSideEffectImports => {
                self.no_unchecked_side_effect_imports.clear();
            }
            TestVariationProp::PreserveConstEnums => self.preserve_const_enums.clear(),
            TestVariationProp::ResolveJsonModule => self.resolve_json_module.clear(),
            TestVariationProp::ResolvePackageJsonExports => {
                self.resolve_package_json_exports.clear();
            }
            TestVariationProp::Strict => self.strict.clear(),
            TestVariationProp::StrictBuiltinIteratorReturn => {
                self.strict_builtin_iterator_return.clear();
            }
            TestVariationProp::StrictNullChecks => self.strict_null_checks.clear(),
            TestVariationProp::Target => self.target.clear(),
            TestVariationProp::UseDefineForClassFields => self.use_define_for_class_fields.clear(),
            TestVariationProp::UseUnknownInCatchVariables => {
                self.use_unknown_in_catch_variables.clear();
            }
            TestVariationProp::VerbatimModuleSyntax => self.verbatim_module_syntax.clear(),
        }
    }

    pub fn iter(&self) -> VariationIter<'_> {
        VariationIter::new(self)
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct TestVariant<'a> {
    pub name: String,
    pub allow_arbitrary_extensions: Option<&'a str>,
    pub allow_importing_ts_extensions: Option<&'a str>,
    pub allow_js: Option<&'a str>,
    pub es_module_interop: Option<&'a str>,
    pub exact_optional_property_types: Option<&'a str>,
    pub isolated_modules: Option<&'a str>,
    pub jsx: Option<&'a str>,
    pub module: Option<&'a str>,
    pub module_detection: Option<&'a str>,
    pub module_resolution: Option<&'a str>,
    pub no_emit: Option<&'a str>,
    pub no_implicit_any: Option<&'a str>,
    pub no_implicit_override: Option<&'a str>,
    pub no_property_access_from_index_signature: Option<&'a str>,
    pub no_unchecked_indexed_access: Option<&'a str>,
    pub no_unchecked_side_effect_imports: Option<&'a str>,
    pub preserve_const_enums: Option<&'a str>,
    pub resolve_json_module: Option<&'a str>,
    pub resolve_package_json_exports: Option<&'a str>,
    pub strict: Option<&'a str>,
    pub strict_builtin_iterator_return: Option<&'a str>,
    pub strict_null_checks: Option<&'a str>,
    pub target: Option<&'a str>,
    pub use_define_for_class_fields: Option<&'a str>,
    pub use_unknown_in_catch_variables: Option<&'a str>,
    pub verbatim_module_syntax: Option<&'a str>,
}

impl<'a> TestVariant<'a> {
    fn set(&mut self, prop: TestVariationProp, value: Option<&'a str>) {
        match prop {
            TestVariationProp::AllowArbitraryExtensions => self.allow_arbitrary_extensions = value,
            TestVariationProp::AllowImportingTsExtensions => {
                self.allow_importing_ts_extensions = value;
            }
            TestVariationProp::AllowJS => self.allow_js = value,
            TestVariationProp::ESModuleInterop => self.es_module_interop = value,
            TestVariationProp::ExactOptionalPropertyTypes => {
                self.exact_optional_property_types = value;
            }
            TestVariationProp::IsolatedModules => self.isolated_modules = value,
            TestVariationProp::Jsx => self.jsx = value,
            TestVariationProp::Module => self.module = value,
            TestVariationProp::ModuleDetection => self.module_detection = value,
            TestVariationProp::ModuleResolution => self.module_resolution = value,
            TestVariationProp::NoEmit => self.no_emit = value,
            TestVariationProp::NoImplicitAny => self.no_implicit_any = value,
            TestVariationProp::NoImplicitOverride => self.no_implicit_override = value,
            TestVariationProp::NoPropertyAccessFromIndexSignature => {
                self.no_property_access_from_index_signature = value;
            }
            TestVariationProp::NoUncheckedIndexedAccess => self.no_unchecked_indexed_access = value,
            TestVariationProp::NoUncheckedSideEffectImports => {
                self.no_unchecked_side_effect_imports = value;
            }
            TestVariationProp::PreserveConstEnums => self.preserve_const_enums = value,
            TestVariationProp::ResolveJsonModule => self.resolve_json_module = value,
            TestVariationProp::ResolvePackageJsonExports => {
                self.resolve_package_json_exports = value;
            }
            TestVariationProp::Strict => self.strict = value,
            TestVariationProp::StrictBuiltinIteratorReturn => {
                self.strict_builtin_iterator_return = value;
            }
            TestVariationProp::StrictNullChecks => self.strict_null_checks = value,
            TestVariationProp::Target => self.target = value,
            TestVariationProp::UseDefineForClassFields => self.use_define_for_class_fields = value,
            TestVariationProp::UseUnknownInCatchVariables => {
                self.use_unknown_in_catch_variables = value;
            }
            TestVariationProp::VerbatimModuleSyntax => self.verbatim_module_syntax = value,
        }
    }

    fn get(&self, prop: TestVariationProp) -> Option<&'_ str> {
        match prop {
            TestVariationProp::AllowArbitraryExtensions => self.allow_arbitrary_extensions,
            TestVariationProp::AllowImportingTsExtensions => self.allow_importing_ts_extensions,
            TestVariationProp::AllowJS => self.allow_js,
            TestVariationProp::ESModuleInterop => self.es_module_interop,
            TestVariationProp::ExactOptionalPropertyTypes => self.exact_optional_property_types,
            TestVariationProp::IsolatedModules => self.isolated_modules,
            TestVariationProp::Jsx => self.jsx,
            TestVariationProp::Module => self.module,
            TestVariationProp::ModuleDetection => self.module_detection,
            TestVariationProp::ModuleResolution => self.module_resolution,
            TestVariationProp::NoEmit => self.no_emit,
            TestVariationProp::NoImplicitAny => self.no_implicit_any,
            TestVariationProp::NoImplicitOverride => self.no_implicit_override,
            TestVariationProp::NoPropertyAccessFromIndexSignature => {
                self.no_property_access_from_index_signature
            }
            TestVariationProp::NoUncheckedIndexedAccess => self.no_unchecked_indexed_access,
            TestVariationProp::NoUncheckedSideEffectImports => {
                self.no_unchecked_side_effect_imports
            }
            TestVariationProp::PreserveConstEnums => self.preserve_const_enums,
            TestVariationProp::ResolveJsonModule => self.resolve_json_module,
            TestVariationProp::ResolvePackageJsonExports => self.resolve_package_json_exports,
            TestVariationProp::Strict => self.strict,
            TestVariationProp::StrictBuiltinIteratorReturn => self.strict_builtin_iterator_return,
            TestVariationProp::StrictNullChecks => self.strict_null_checks,
            TestVariationProp::Target => self.target,
            TestVariationProp::UseDefineForClassFields => self.use_define_for_class_fields,
            TestVariationProp::UseUnknownInCatchVariables => self.use_unknown_in_catch_variables,
            TestVariationProp::VerbatimModuleSyntax => self.verbatim_module_syntax,
        }
    }

    fn update_name(&mut self, name_props: &[TestVariationProp]) {
        let components: Vec<_> = name_props
            .iter()
            .map(|&p| format!("{}={}", <&str>::from(p), self.get(p).unwrap()))
            .collect();
        self.name = if components.is_empty() {
            String::new()
        } else {
            format!("({})", components.join(","))
        };
    }
}

#[derive(Debug)]
struct RestartableIterator<'a> {
    arr: &'a Vec<CompactString>,
    idx: u8,
}

impl<'a> RestartableIterator<'a> {
    fn new(arr: &'a Vec<CompactString>) -> Self {
        Self { arr, idx: 0 }
    }
}

impl<'a> Iterator for RestartableIterator<'a> {
    type Item = Option<&'a str>;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.arr.get(self.idx as usize).map(|x| Some(x.as_str()));
        if result.is_none() {
            self.idx = 0;
        } else {
            self.idx += 1;
        }

        result
    }
}

#[derive(Debug)]
pub struct VariationIter<'a> {
    name_props: Vec<TestVariationProp>,
    template: TestVariant<'a>,
    iter: Vec<RestartableIterator<'a>>,
    done: bool,
}

impl<'a> VariationIter<'a> {
    fn new(variations: &'a TestVariations) -> Self {
        let mut result = Self {
            name_props: vec![],
            template: TestVariant::default(),
            iter: vec![],
            done: false,
        };

        for &prop in TEST_VARIATION_PROPS {
            let arr = variations.get(prop);
            match arr.len() {
                0 => {}
                1 => {
                    result.template.set(prop, Some(arr[0].as_str()));
                }
                _ => {
                    result.name_props.push(prop);
                    let mut iter = RestartableIterator::new(arr);
                    result.template.set(prop, iter.next().unwrap());
                    result.iter.push(iter);
                }
            }
        }

        result.template.update_name(&result.name_props);

        result
    }
}

impl<'a> Iterator for VariationIter<'a> {
    type Item = TestVariant<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iter.is_empty() {
            if self.done {
                None
            } else {
                self.done = true;
                Some(self.template.clone())
            }
        } else if self.done {
            for i in (0..self.name_props.len()).rev() {
                let Some(value) = self.iter[i].next() else {
                    // println!("{:?} -> None", self.name_props[i]);
                    continue;
                };

                // println!("{:?} -> {:?}", self.name_props[i], value);

                self.template.set(self.name_props[i], value);
                for i in i + 1..self.name_props.len() {
                    let value = self.iter[i].next();
                    // println!("[RESTART] {:?} -> {:?}", self.name_props[i], value);
                    self.template.set(self.name_props[i], value.unwrap());
                }

                let mut result = self.template.clone();
                result.update_name(&self.name_props);
                return Some(result);
            }

            None
        } else {
            self.done = true;
            Some(self.template.clone())
        }
    }
}

oxc_index::define_index_type! {
  pub struct FileId = u8;
}

#[derive(Debug, PartialEq, Eq)]
pub struct TestUnit<'a> {
    pub path: &'a Path,
    pub settings: TestSettings,
    pub variations: TestVariations,
    pub file_names: IndexVec<FileId, &'a str>,
    pub file_contents: IndexVec<FileId, &'a str>,
    pub symlinks: FxHashMap<&'a str, &'a str>,
}

impl<'a> TestUnit<'a> {
    /// # Panics
    pub fn parse(path: &'a Path, data: &'a [u8]) -> Self {
        let mut result = Self {
            path,
            settings: TestSettings::default(),
            variations: TestVariations::default(),
            file_names: IndexVec::default(),
            file_contents: IndexVec::default(),
            symlinks: FxHashMap::default(),
        };

        let mut iter = memchr_iter(b'\n', data);
        let mut line_start = 0usize;
        let mut file_start = None;
        let mut file_name = path
            .file_name()
            .expect("test unit path to be a file")
            .to_str()
            .expect("test unit file name to be UTF8");

        while line_start < data.len() {
            let eol = iter.next().unwrap_or_else(|| data.len() - 1);
            let line = &data[line_start..=eol];
            // println!("line: {}", str::from_utf8(line).unwrap().escape_debug());

            if let [b'/', b'/', rest @ ..] = line {
                let rest = trim_space_start(rest);
                if rest.len() >= 4 && rest[0] == b'@' {
                    if let Some(name_end) = memchr(b':', &rest[2..]) {
                        if let Some(content_start) = file_start {
                            // println!("file complete: {file_name}");
                            result.file_names.push(file_name);
                            result.file_contents.push(
                                str::from_utf8(&data[content_start..line_start])
                                    .expect("file content to be UTF8"),
                            );
                            file_start = None;
                        }

                        // SAFETY: index is a result of a string search
                        #[expect(unsafe_code)]
                        let (name, rest) = unsafe { rest[1..].split_at_unchecked(name_end + 1) };
                        let name = trim_space_end(name);
                        let value_end = rest.len()
                            - if rest[rest.len() - 2] == b'\r' {
                                2
                            } else {
                                usize::from(rest[rest.len() - 1] == b'\n')
                            };
                        let value = trim_space_start(&rest[1..value_end]);

                        // println!(
                        //   "option: {} = {}",
                        //   str::from_utf8(name).unwrap().escape_debug(),
                        //   str::from_utf8(value).unwrap().escape_debug()
                        // );

                        match &name.to_ascii_lowercase()[..] {
                            b"filename" => {
                                file_name = str::from_utf8(value).expect("filename to be UTF8");
                                file_start = Some(eol + 1);
                            }
                            b"link" => {
                                let separator = memchr(b' ', value)
                                    .expect("symlink arguments should be separated by space");
                                let from = str::from_utf8(&value[..separator])
                                    .expect("symlink argument to be UTF8");
                                let to = str::from_utf8(&value[separator + 1..])
                                    .expect("symlink argument to be UTF8");
                                result.symlinks.insert(from, to);
                            }
                            b"baseurl" => {
                                result.settings.base_url = Some(
                                    CompactString::from_utf8(value).expect("baseUrl to be UTF8"),
                                );
                            }
                            b"noimplicitreferences" => {
                                result.settings.no_implicit_references = match &value
                                    .to_ascii_lowercase()[..]
                                {
                                    b"true" => true,
                                    b"false" => false,
                                    _ => panic!(
                                        "Unknown value for noImplicitReferences: {}",
                                        str::from_utf8(value).unwrap_or_default().escape_debug()
                                    ),
                                };
                            }
                            b"includebuiltfile" => {
                                result.settings.include_built_file = Some(
                                    CompactString::from_utf8(value)
                                        .expect("includeBuildFile to be UTF8"),
                                );
                            }
                            b"libfiles" => {
                                let mut lib_files = vec![];
                                let mut start = 0usize;
                                for separator in
                                    memchr_iter(b',', value).chain(iter::once(value.len()))
                                {
                                    let name = CompactString::from_utf8(trim_space(
                                        &value[start..separator],
                                    ))
                                    .expect("libFile to be UTF8");
                                    if !name.is_empty() {
                                        lib_files.push(name);
                                    }
                                    start = separator + 1;
                                }

                                result.settings.lib_files = Some(lib_files);
                            }
                            b"notypesandsymbols" => {
                                result.settings.no_types_and_symbols = match &value
                                    .to_ascii_lowercase()[..]
                                {
                                    b"true" => true,
                                    b"false" => false,
                                    _ => panic!(
                                        "Unknown value for noTypeAndSymbols: {}",
                                        str::from_utf8(value).unwrap_or_default().escape_debug()
                                    ),
                                };
                            }
                            prop => {
                                if let Ok(prop) = TestVariationProp::try_from(prop) {
                                    result.variations.clear(prop);
                                    if value == b"*" {
                                        for value in prop.expand_wildcard() {
                                            result.variations.push(prop, value);
                                        }
                                    } else {
                                        let mut start = 0usize;
                                        for separator in
                                            memchr_iter(b',', value).chain(iter::once(value.len()))
                                        {
                                            let value = CompactString::from_utf8(trim_space(
                                                &value[start..separator],
                                            ))
                                            .expect("Test option to be UTF8");
                                            if !value.is_empty() {
                                                result.variations.push(prop, value);
                                            }
                                            start = separator + 1;
                                        }
                                    }
                                }
                                // println!("unknown option: {}", str::from_utf8(name).unwrap().escape_debug());
                            }
                        }
                    }
                }
            } else if file_start.is_none() && !line.is_empty() && line != b"\n" && line != b"\r\n" {
                // println!(
                //   "file start @ {}: {}",
                //   line_start,
                //   str::from_utf8(&data[(line_start.saturating_sub(10))..(line_start + 11).min(data.len())])
                //     .unwrap()
                //     .escape_debug()
                // );
                file_start = Some(line_start);
            }

            line_start = eol + 1;
        }

        if let Some(file_start) = file_start {
            // println!("file complete: {file_name}");
            let Ok(content) = str::from_utf8(&data[file_start..]) else {
                panic!(
                    "Expected file content to be UTF8:\n  path: {}\n  file_name: {}",
                    path.display(),
                    file_name
                );
            };

            result.file_names.push(file_name);
            result.file_contents.push(content);
        } else if result.file_names.is_empty() {
            result.file_names.push(file_name);
            result.file_contents.push("");
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod restartable_iter {
        use compact_str::ToCompactString;

        use super::RestartableIterator;

        #[test]
        fn restarted_after_none() {
            let arr =
                vec!["a".to_compact_string(), "b".to_compact_string(), "c".to_compact_string()];
            let mut iter = RestartableIterator::new(&arr);

            assert_eq!(iter.next(), Some(Some("a")));
            assert_eq!(iter.next(), Some(Some("b")));
            assert_eq!(iter.next(), Some(Some("c")));
            assert_eq!(iter.next(), None);
            assert_eq!(iter.next(), Some(Some("a")));
        }
    }

    mod parsing {
        use oxc_index::index_vec;
        use std::{path::PathBuf, str::FromStr};

        use super::*;

        #[test]
        fn single_file() {
            let path = PathBuf::from_str("tests/cases/unit1.ts").unwrap();
            let data = br"export const foo = 5;";

            let test_unit = TestUnit::parse(&path, data);
            assert_eq!(
                test_unit,
                TestUnit {
                    path: &path,
                    settings: TestSettings::default(),
                    variations: TestVariations::default(),
                    file_names: index_vec!["unit1.ts"],
                    file_contents: index_vec!["export const foo = 5;"],
                    symlinks: FxHashMap::default(),
                }
            );
        }

        #[test]
        fn single_file_with_options() {
            let path = PathBuf::from_str("tests/cases/unit1.ts").unwrap();
            let data = br"// @baseUrl: .
// @noTypesAndSymbols: true
// @noImplicitReferences: true
// @includeBuiltFile: lib.d.ts
// @libFiles: lib.d.ts,react.d.ts
export const foo = 5;";

            let test_unit = TestUnit::parse(&path, data);
            assert_eq!(
                test_unit,
                TestUnit {
                    path: &path,
                    settings: TestSettings {
                        no_types_and_symbols: true,
                        base_url: Some(".".into()),
                        no_implicit_references: true,
                        include_built_file: Some("lib.d.ts".into()),
                        lib_files: Some(vec!["lib.d.ts".into(), "react.d.ts".into()])
                    },
                    variations: TestVariations::default(),
                    file_names: index_vec!["unit1.ts"],
                    file_contents: index_vec!["export const foo = 5;"],
                    symlinks: FxHashMap::default(),
                }
            );
        }

        #[test]
        fn single_file_with_variations() {
            let path = PathBuf::from_str("tests/cases/unit1.ts").unwrap();
            let data = br"// @module: es5, preserve
export const foo = 5;";

            let test_unit = TestUnit::parse(&path, data);
            assert_eq!(
                test_unit,
                TestUnit {
                    path: &path,
                    settings: TestSettings::default(),
                    variations: TestVariations {
                        module: vec!["es5".into(), "preserve".into()],
                        ..Default::default()
                    },
                    file_names: index_vec!["unit1.ts"],
                    file_contents: index_vec!["export const foo = 5;"],
                    symlinks: FxHashMap::default(),
                }
            );
        }

        #[test]
        fn single_file_with_name() {
            let path = PathBuf::from_str("tests/cases/unit1.ts").unwrap();
            let data = br"// @fileName: /a.js
export const foo = 5;";

            let test_unit = TestUnit::parse(&path, data);
            assert_eq!(
                test_unit,
                TestUnit {
                    path: &path,
                    settings: TestSettings::default(),
                    variations: TestVariations::default(),
                    file_names: index_vec!["/a.js"],
                    file_contents: index_vec!["export const foo = 5;"],
                    symlinks: FxHashMap::default(),
                }
            );
        }

        #[test]
        fn multiple_files() {
            let path = PathBuf::from_str("tests/cases/unit1.ts").unwrap();
            let data = br"// @fileName: /a.js
export const foo = 5;
//@filename:b.js
export const b = 123;

// @Filename: /some/file.ts
export function bar() {}";

            let test_unit = TestUnit::parse(&path, data);
            assert_eq!(
                test_unit,
                TestUnit {
                    path: &path,
                    settings: TestSettings::default(),
                    variations: TestVariations::default(),
                    file_names: index_vec!["/a.js", "b.js", "/some/file.ts"],
                    file_contents: index_vec![
                        r"export const foo = 5;
",
                        r"export const b = 123;

",
                        r"export function bar() {}"
                    ],
                    symlinks: FxHashMap::default(),
                }
            );
        }

        #[test]
        fn files_and_symlinks() {
            let path = PathBuf::from_str("tests/cases/unit1.ts").unwrap();
            let data = br"// @fileName: /a.js
export const foo = 5;

// @link: foo bar
// @Link: ab1 ab2

//@filename:b.js
export const b = 123;
//@link: a123 b123

// @Filename: /some/file.ts
export function bar() {}
// @link: q1 q2";

            let test_unit = TestUnit::parse(&path, data);
            assert_eq!(
                test_unit,
                TestUnit {
                    path: &path,
                    settings: TestSettings::default(),
                    variations: TestVariations::default(),
                    file_names: index_vec!["/a.js", "b.js", "/some/file.ts"],
                    file_contents: index_vec![
                        r"export const foo = 5;

",
                        r"export const b = 123;
",
                        r"export function bar() {}
"
                    ],
                    symlinks: vec![("foo", "bar"), ("ab1", "ab2"), ("a123", "b123"), ("q1", "q2")]
                        .into_iter()
                        .collect(),
                }
            );
        }

        #[test]
        fn empty_files() {
            let path = PathBuf::from_str("tests/cases/unit1.ts").unwrap();
            let data = br"// @fileName: /a.js

//@filename:b.js

// @Filename: /some/file.ts
/// foo";

            let test_unit = TestUnit::parse(&path, data);
            assert_eq!(
                test_unit,
                TestUnit {
                    path: &path,
                    settings: TestSettings::default(),
                    variations: TestVariations::default(),
                    file_names: index_vec!["/a.js", "b.js", "/some/file.ts"],
                    file_contents: index_vec![
                        r"
", r"
", r"/// foo"
                    ],
                    symlinks: FxHashMap::default(),
                }
            );
        }

        #[test]
        fn duplicated_options() {
            let path = PathBuf::from_str("tests/cases/unit1.ts").unwrap();
            let data = br"// @strict: true
// @declaration: true

// @strict: true
// @declaration: true
export const foo = 5;";

            let test_unit = TestUnit::parse(&path, data);
            assert_eq!(
                test_unit,
                TestUnit {
                    path: &path,
                    settings: TestSettings::default(),
                    variations: TestVariations {
                        strict: vec!["true".into()],
                        ..Default::default()
                    },
                    file_names: index_vec!["unit1.ts"],
                    file_contents: index_vec!["export const foo = 5;"],
                    symlinks: FxHashMap::default(),
                }
            );
        }

        #[test]
        fn wildcard_options() {
            let path = PathBuf::from_str("tests/cases/unit1.ts").unwrap();
            let data = br"// @target: esnext
// @strict: true
// @strictBuiltinIteratorReturn: *
export const foo = 5;";

            let test_unit = TestUnit::parse(&path, data);
            assert_eq!(
                test_unit,
                TestUnit {
                    path: &path,
                    settings: TestSettings::default(),
                    variations: TestVariations {
                        strict: vec!["true".into()],
                        target: vec!["esnext".into()],
                        strict_builtin_iterator_return: vec!["true".into(), "false".into()],
                        ..Default::default()
                    },
                    file_names: index_vec!["unit1.ts"],
                    file_contents: index_vec!["export const foo = 5;"],
                    symlinks: FxHashMap::default(),
                }
            );
        }
    }

    mod variant_iter {
        use compact_str::ToCompactString;

        use super::*;

        #[test]
        fn single_var1() {
            let variations = TestVariations {
                allow_arbitrary_extensions: vec![
                    "true".to_compact_string(),
                    "false".to_compact_string(),
                ],
                ..Default::default()
            };
            let result: Vec<_> = variations.iter().collect();
            assert_eq!(
                result,
                vec![
                    TestVariant {
                        name: "(allowarbitraryextensions=true)".to_string(),
                        allow_arbitrary_extensions: Some("true"),
                        ..Default::default()
                    },
                    TestVariant {
                        name: "(allowarbitraryextensions=false)".to_string(),
                        allow_arbitrary_extensions: Some("false"),
                        ..Default::default()
                    },
                ]
            );
        }

        #[test]
        fn single_var2() {
            let variations = TestVariations {
                module: vec!["commonjs".to_compact_string(), "umd".to_compact_string()],
                ..Default::default()
            };
            let result: Vec<_> = variations.iter().collect();
            assert_eq!(
                result,
                vec![
                    TestVariant {
                        name: "(module=commonjs)".to_string(),
                        module: Some("commonjs"),
                        ..Default::default()
                    },
                    TestVariant {
                        name: "(module=umd)".to_string(),
                        module: Some("umd"),
                        ..Default::default()
                    },
                ]
            );
        }

        #[test]
        fn double_var() {
            let variations = TestVariations {
                module: vec!["commonjs".to_compact_string(), "umd".to_compact_string()],
                target: vec!["es5".to_compact_string(), "es6".to_compact_string()],
                ..Default::default()
            };
            let result: Vec<_> = variations.iter().collect();
            assert_eq!(
                result,
                vec![
                    TestVariant {
                        name: "(module=commonjs,target=es5)".to_string(),
                        module: Some("commonjs"),
                        target: Some("es5"),
                        ..Default::default()
                    },
                    TestVariant {
                        name: "(module=commonjs,target=es6)".to_string(),
                        module: Some("commonjs"),
                        target: Some("es6"),
                        ..Default::default()
                    },
                    TestVariant {
                        name: "(module=umd,target=es5)".to_string(),
                        module: Some("umd"),
                        target: Some("es5"),
                        ..Default::default()
                    },
                    TestVariant {
                        name: "(module=umd,target=es6)".to_string(),
                        module: Some("umd"),
                        target: Some("es6"),
                        ..Default::default()
                    },
                ]
            );
        }
    }
}
