use oxc::{
    ast::AstKind,
    semantic::Semantic,
    span::{GetSpan, Span},
};
use oxc_ast_visit::Visit;

use crate::baseline::types_baseline::{Assertion, TypeBaselineFile};

pub struct TypeVisitor<'a> {
    pub semantic: &'a Semantic<'a>,
    pub baseline: &'a TypeBaselineFile<'a>,
}

impl TypeVisitor<'_> {
    pub fn run(&self) {
        let source_text = self.semantic.source_text();
        println!(
            "{}",
            self.baseline
                .assertions
                .iter()
                .flat_map(|x| x.iter().map(|x| format!("{} : {}\n", x.expr, x.expected_type)))
                .collect::<String>()
        );
        let assertions = self.baseline.assertions.iter().flat_map(|x| x.iter());
        let mut visitor = TypeVisitorImpl { source_text, assertions, depth: 2 };
        let AstKind::Program(program) =
            self.semantic.nodes().root_node().expect("root node to exist").kind()
        else {
            panic!("Expected root AST node to be Program");
        };
        visitor.visit_program(program);
    }
}

struct TypeVisitorImpl<'a, T: Iterator<Item = &'a Assertion<'a>>> {
    source_text: &'a str,
    assertions: T,
    depth: usize,
}

impl<'a, T: Iterator<Item = &'a Assertion<'a>>> TypeVisitorImpl<'a, T> {
    fn assert(&mut self, span: Span, node_type: &str, kind: &str) {
        let text = span.source_text(self.source_text);

        let Some(assertion) = self.assertions.next() else {
            panic!("Expected assertion for:\n  source: {}", text.escape_debug());
        };

        assert!(
            (assertion.expr == text),
            "Expected assertion expression to match:\n  kind: {}\n  context: {}\n  expected: {}\n    actual: {}",
            kind,
            Span::new(
                span.start.saturating_sub(10),
                (span.end + 10).min(u32::try_from(self.source_text.len()).unwrap())
            )
            .source_text(self.source_text)
            .escape_debug(),
            assertion.expr.escape_debug(),
            text.escape_debug(),
        );

        println!(
            "{}[91m>[0m {:<width$} {} : {}",
            " ".repeat(self.depth),
            kind,
            text.escape_debug(),
            node_type,
            width = 50 - self.depth,
        );
    }
}

impl<'a, T: Iterator<Item = &'a Assertion<'a>>> Visit<'a> for TypeVisitorImpl<'a, T> {
    fn visit_expression(&mut self, it: &oxc::ast::ast::Expression<'a>) {
        let span = GetSpan::span(it);
        println!("{}[96mvisit_expression([90m{:?}[96m)[0m", " ".repeat(self.depth), span);
        let node_type = "any";

        self.assert(span, node_type, &AstKind::from_expression(it).debug_name());

        match it {
            oxc::ast::ast::Expression::Identifier(_) => {}
            _ => {
                oxc_ast_visit::walk::walk_expression(self, it);
            }
        }
    }

    fn visit_identifier_reference(&mut self, it: &oxc::ast::ast::IdentifierReference<'a>) {
        let span = GetSpan::span(it);
        println!(
            "{}[96mvisit_identifier_reference([90m{:?}[96m)[0m",
            " ".repeat(self.depth),
            span
        );
        let node_type = "any";

        self.assert(span, node_type, &AstKind::IdentifierReference(it).debug_name());

        oxc_ast_visit::walk::walk_identifier_reference(self, it);
    }

    fn visit_identifier_name(&mut self, it: &oxc::ast::ast::IdentifierName<'a>) {
        if it.name == "constructor" {
            return;
        }

        let span = GetSpan::span(it);
        println!("{}[96mvisit_identifier_name([90m{:?}[96m)[0m", " ".repeat(self.depth), span);
        let node_type = "any";

        self.assert(span, node_type, &AstKind::IdentifierName(it).debug_name());

        oxc_ast_visit::walk::walk_identifier_name(self, it);
    }

    fn visit_binding_identifier(&mut self, it: &oxc::ast::ast::BindingIdentifier<'a>) {
        // Span includes type annotation, we should shrink it to include just the name
        let span = {
            let span = GetSpan::span(it);
            let len = u32::try_from(it.name.len()).expect("identifier length to be within u32");
            if span.size() <= len { span } else { Span::new(span.start, span.start + len) }
        };

        println!(
            "{}[96mvisit_binding_identifier([90m{:?}[96m)[0m",
            " ".repeat(self.depth),
            span
        );
        let node_type = "any";

        self.assert(span, node_type, &AstKind::BindingIdentifier(it).debug_name());
        oxc_ast_visit::walk::walk_binding_identifier(self, it);
    }

    fn visit_private_identifier(&mut self, it: &oxc::ast::ast::PrivateIdentifier<'a>) {
        let span = GetSpan::span(it);
        println!(
            "{}[96mvisit_private_identifier([90m{:?}[96m)[0m",
            " ".repeat(self.depth),
            span
        );
        let node_type = "any";

        self.assert(span, node_type, &AstKind::PrivateIdentifier(it).debug_name());

        oxc_ast_visit::walk::walk_private_identifier(self, it);
    }

    fn visit_jsx_identifier(&mut self, it: &oxc::ast::ast::JSXIdentifier<'a>) {
        let span = GetSpan::span(it);
        println!("{}[96mvisit_jsx_identifier([90m{:?}[96m)[0m", " ".repeat(self.depth), span);
        let node_type = "any";

        self.assert(span, node_type, &AstKind::JSXIdentifier(it).debug_name());

        oxc_ast_visit::walk::walk_jsx_identifier(self, it);
    }

    fn visit_ts_type_name(&mut self, _it: &oxc::ast::ast::TSTypeName<'a>) {
        // oxc_ast_visit::walk::walk_ts_type_name(self, it);
    }

    fn enter_node(&mut self, kind: AstKind<'a>) {
        println!("{}[90m{}[0m", " ".repeat(self.depth), kind.debug_name());
        self.depth += 1;
    }

    fn leave_node(&mut self, _kind: AstKind<'a>) {
        self.depth -= 1;
    }

    fn enter_scope(
        &mut self,
        _flags: oxc::semantic::ScopeFlags,
        _scope_id: &std::cell::Cell<Option<oxc::semantic::ScopeId>>,
    ) {
    }

    fn leave_scope(&mut self) {}

    fn visit_program(&mut self, it: &oxc::ast::ast::Program<'a>) {
        oxc_ast_visit::walk::walk_program(self, it);
    }

    fn visit_label_identifier(&mut self, it: &oxc::ast::ast::LabelIdentifier<'a>) {
        oxc_ast_visit::walk::walk_label_identifier(self, it);
    }

    fn visit_this_expression(&mut self, it: &oxc::ast::ast::ThisExpression) {
        oxc_ast_visit::walk::walk_this_expression(self, it);
    }

    fn visit_array_expression(&mut self, it: &oxc::ast::ast::ArrayExpression<'a>) {
        oxc_ast_visit::walk::walk_array_expression(self, it);
    }

    fn visit_array_expression_element(&mut self, it: &oxc::ast::ast::ArrayExpressionElement<'a>) {
        oxc_ast_visit::walk::walk_array_expression_element(self, it);
    }

    fn visit_elision(&mut self, it: &oxc::ast::ast::Elision) {
        oxc_ast_visit::walk::walk_elision(self, it);
    }

    fn visit_object_expression(&mut self, it: &oxc::ast::ast::ObjectExpression<'a>) {
        oxc_ast_visit::walk::walk_object_expression(self, it);
    }

    fn visit_object_property_kind(&mut self, it: &oxc::ast::ast::ObjectPropertyKind<'a>) {
        oxc_ast_visit::walk::walk_object_property_kind(self, it);
    }

    fn visit_object_property(&mut self, it: &oxc::ast::ast::ObjectProperty<'a>) {
        oxc_ast_visit::walk::walk_object_property(self, it);
    }

    fn visit_property_key(&mut self, it: &oxc::ast::ast::PropertyKey<'a>) {
        oxc_ast_visit::walk::walk_property_key(self, it);
    }

    fn visit_template_literal(&mut self, it: &oxc::ast::ast::TemplateLiteral<'a>) {
        oxc_ast_visit::walk::walk_template_literal(self, it);
    }

    fn visit_tagged_template_expression(
        &mut self,
        it: &oxc::ast::ast::TaggedTemplateExpression<'a>,
    ) {
        oxc_ast_visit::walk::walk_tagged_template_expression(self, it);
    }

    fn visit_template_element(&mut self, it: &oxc::ast::ast::TemplateElement<'a>) {
        oxc_ast_visit::walk::walk_template_element(self, it);
    }

    fn visit_member_expression(&mut self, it: &oxc::ast::ast::MemberExpression<'a>) {
        oxc_ast_visit::walk::walk_member_expression(self, it);
    }

    fn visit_computed_member_expression(
        &mut self,
        it: &oxc::ast::ast::ComputedMemberExpression<'a>,
    ) {
        oxc_ast_visit::walk::walk_computed_member_expression(self, it);
    }

    fn visit_static_member_expression(&mut self, it: &oxc::ast::ast::StaticMemberExpression<'a>) {
        oxc_ast_visit::walk::walk_static_member_expression(self, it);
    }

    fn visit_private_field_expression(&mut self, it: &oxc::ast::ast::PrivateFieldExpression<'a>) {
        oxc_ast_visit::walk::walk_private_field_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &oxc::ast::ast::CallExpression<'a>) {
        oxc_ast_visit::walk::walk_call_expression(self, it);
    }

    fn visit_new_expression(&mut self, it: &oxc::ast::ast::NewExpression<'a>) {
        oxc_ast_visit::walk::walk_new_expression(self, it);
    }

    fn visit_meta_property(&mut self, it: &oxc::ast::ast::MetaProperty<'a>) {
        oxc_ast_visit::walk::walk_meta_property(self, it);
    }

    fn visit_spread_element(&mut self, it: &oxc::ast::ast::SpreadElement<'a>) {
        oxc_ast_visit::walk::walk_spread_element(self, it);
    }

    fn visit_argument(&mut self, it: &oxc::ast::ast::Argument<'a>) {
        oxc_ast_visit::walk::walk_argument(self, it);
    }

    fn visit_update_expression(&mut self, it: &oxc::ast::ast::UpdateExpression<'a>) {
        oxc_ast_visit::walk::walk_update_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &oxc::ast::ast::UnaryExpression<'a>) {
        oxc_ast_visit::walk::walk_unary_expression(self, it);
    }

    fn visit_binary_expression(&mut self, it: &oxc::ast::ast::BinaryExpression<'a>) {
        oxc_ast_visit::walk::walk_binary_expression(self, it);
    }

    fn visit_private_in_expression(&mut self, it: &oxc::ast::ast::PrivateInExpression<'a>) {
        oxc_ast_visit::walk::walk_private_in_expression(self, it);
    }

    fn visit_logical_expression(&mut self, it: &oxc::ast::ast::LogicalExpression<'a>) {
        oxc_ast_visit::walk::walk_logical_expression(self, it);
    }

    fn visit_conditional_expression(&mut self, it: &oxc::ast::ast::ConditionalExpression<'a>) {
        oxc_ast_visit::walk::walk_conditional_expression(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &oxc::ast::ast::AssignmentExpression<'a>) {
        oxc_ast_visit::walk::walk_assignment_expression(self, it);
    }

    fn visit_assignment_target(&mut self, it: &oxc::ast::ast::AssignmentTarget<'a>) {
        oxc_ast_visit::walk::walk_assignment_target(self, it);
    }

    fn visit_simple_assignment_target(&mut self, it: &oxc::ast::ast::SimpleAssignmentTarget<'a>) {
        oxc_ast_visit::walk::walk_simple_assignment_target(self, it);
    }

    fn visit_assignment_target_pattern(&mut self, it: &oxc::ast::ast::AssignmentTargetPattern<'a>) {
        oxc_ast_visit::walk::walk_assignment_target_pattern(self, it);
    }

    fn visit_array_assignment_target(&mut self, it: &oxc::ast::ast::ArrayAssignmentTarget<'a>) {
        oxc_ast_visit::walk::walk_array_assignment_target(self, it);
    }

    fn visit_object_assignment_target(&mut self, it: &oxc::ast::ast::ObjectAssignmentTarget<'a>) {
        oxc_ast_visit::walk::walk_object_assignment_target(self, it);
    }

    fn visit_assignment_target_rest(&mut self, it: &oxc::ast::ast::AssignmentTargetRest<'a>) {
        oxc_ast_visit::walk::walk_assignment_target_rest(self, it);
    }

    fn visit_assignment_target_maybe_default(
        &mut self,
        it: &oxc::ast::ast::AssignmentTargetMaybeDefault<'a>,
    ) {
        oxc_ast_visit::walk::walk_assignment_target_maybe_default(self, it);
    }

    fn visit_assignment_target_with_default(
        &mut self,
        it: &oxc::ast::ast::AssignmentTargetWithDefault<'a>,
    ) {
        oxc_ast_visit::walk::walk_assignment_target_with_default(self, it);
    }

    fn visit_assignment_target_property(
        &mut self,
        it: &oxc::ast::ast::AssignmentTargetProperty<'a>,
    ) {
        oxc_ast_visit::walk::walk_assignment_target_property(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &oxc::ast::ast::AssignmentTargetPropertyIdentifier<'a>,
    ) {
        oxc_ast_visit::walk::walk_assignment_target_property_identifier(self, it);
    }

    fn visit_assignment_target_property_property(
        &mut self,
        it: &oxc::ast::ast::AssignmentTargetPropertyProperty<'a>,
    ) {
        oxc_ast_visit::walk::walk_assignment_target_property_property(self, it);
    }

    fn visit_sequence_expression(&mut self, it: &oxc::ast::ast::SequenceExpression<'a>) {
        oxc_ast_visit::walk::walk_sequence_expression(self, it);
    }

    fn visit_super(&mut self, it: &oxc::ast::ast::Super) {
        oxc_ast_visit::walk::walk_super(self, it);
    }

    fn visit_await_expression(&mut self, it: &oxc::ast::ast::AwaitExpression<'a>) {
        oxc_ast_visit::walk::walk_await_expression(self, it);
    }

    fn visit_chain_expression(&mut self, it: &oxc::ast::ast::ChainExpression<'a>) {
        oxc_ast_visit::walk::walk_chain_expression(self, it);
    }

    fn visit_chain_element(&mut self, it: &oxc::ast::ast::ChainElement<'a>) {
        oxc_ast_visit::walk::walk_chain_element(self, it);
    }

    fn visit_parenthesized_expression(&mut self, it: &oxc::ast::ast::ParenthesizedExpression<'a>) {
        oxc_ast_visit::walk::walk_parenthesized_expression(self, it);
    }

    fn visit_statement(&mut self, it: &oxc::ast::ast::Statement<'a>) {
        oxc_ast_visit::walk::walk_statement(self, it);
    }

    fn visit_directive(&mut self, it: &oxc::ast::ast::Directive<'a>) {
        oxc_ast_visit::walk::walk_directive(self, it);
    }

    fn visit_hashbang(&mut self, it: &oxc::ast::ast::Hashbang<'a>) {
        oxc_ast_visit::walk::walk_hashbang(self, it);
    }

    fn visit_block_statement(&mut self, it: &oxc::ast::ast::BlockStatement<'a>) {
        oxc_ast_visit::walk::walk_block_statement(self, it);
    }

    fn visit_declaration(&mut self, it: &oxc::ast::ast::Declaration<'a>) {
        oxc_ast_visit::walk::walk_declaration(self, it);
    }

    fn visit_variable_declaration(&mut self, it: &oxc::ast::ast::VariableDeclaration<'a>) {
        oxc_ast_visit::walk::walk_variable_declaration(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &oxc::ast::ast::VariableDeclarator<'a>) {
        oxc_ast_visit::walk::walk_variable_declarator(self, it);
    }

    fn visit_empty_statement(&mut self, it: &oxc::ast::ast::EmptyStatement) {
        oxc_ast_visit::walk::walk_empty_statement(self, it);
    }

    fn visit_expression_statement(&mut self, it: &oxc::ast::ast::ExpressionStatement<'a>) {
        oxc_ast_visit::walk::walk_expression_statement(self, it);
    }

    fn visit_if_statement(&mut self, it: &oxc::ast::ast::IfStatement<'a>) {
        oxc_ast_visit::walk::walk_if_statement(self, it);
    }

    fn visit_do_while_statement(&mut self, it: &oxc::ast::ast::DoWhileStatement<'a>) {
        oxc_ast_visit::walk::walk_do_while_statement(self, it);
    }

    fn visit_while_statement(&mut self, it: &oxc::ast::ast::WhileStatement<'a>) {
        oxc_ast_visit::walk::walk_while_statement(self, it);
    }

    fn visit_for_statement(&mut self, it: &oxc::ast::ast::ForStatement<'a>) {
        oxc_ast_visit::walk::walk_for_statement(self, it);
    }

    fn visit_for_statement_init(&mut self, it: &oxc::ast::ast::ForStatementInit<'a>) {
        oxc_ast_visit::walk::walk_for_statement_init(self, it);
    }

    fn visit_for_in_statement(&mut self, it: &oxc::ast::ast::ForInStatement<'a>) {
        oxc_ast_visit::walk::walk_for_in_statement(self, it);
    }

    fn visit_for_statement_left(&mut self, it: &oxc::ast::ast::ForStatementLeft<'a>) {
        oxc_ast_visit::walk::walk_for_statement_left(self, it);
    }

    fn visit_for_of_statement(&mut self, it: &oxc::ast::ast::ForOfStatement<'a>) {
        oxc_ast_visit::walk::walk_for_of_statement(self, it);
    }

    fn visit_continue_statement(&mut self, it: &oxc::ast::ast::ContinueStatement<'a>) {
        oxc_ast_visit::walk::walk_continue_statement(self, it);
    }

    fn visit_break_statement(&mut self, it: &oxc::ast::ast::BreakStatement<'a>) {
        oxc_ast_visit::walk::walk_break_statement(self, it);
    }

    fn visit_return_statement(&mut self, it: &oxc::ast::ast::ReturnStatement<'a>) {
        oxc_ast_visit::walk::walk_return_statement(self, it);
    }

    fn visit_with_statement(&mut self, it: &oxc::ast::ast::WithStatement<'a>) {
        oxc_ast_visit::walk::walk_with_statement(self, it);
    }

    fn visit_switch_statement(&mut self, it: &oxc::ast::ast::SwitchStatement<'a>) {
        oxc_ast_visit::walk::walk_switch_statement(self, it);
    }

    fn visit_switch_case(&mut self, it: &oxc::ast::ast::SwitchCase<'a>) {
        oxc_ast_visit::walk::walk_switch_case(self, it);
    }

    fn visit_labeled_statement(&mut self, it: &oxc::ast::ast::LabeledStatement<'a>) {
        oxc_ast_visit::walk::walk_labeled_statement(self, it);
    }

    fn visit_throw_statement(&mut self, it: &oxc::ast::ast::ThrowStatement<'a>) {
        oxc_ast_visit::walk::walk_throw_statement(self, it);
    }

    fn visit_try_statement(&mut self, it: &oxc::ast::ast::TryStatement<'a>) {
        oxc_ast_visit::walk::walk_try_statement(self, it);
    }

    fn visit_catch_clause(&mut self, it: &oxc::ast::ast::CatchClause<'a>) {
        oxc_ast_visit::walk::walk_catch_clause(self, it);
    }

    fn visit_catch_parameter(&mut self, it: &oxc::ast::ast::CatchParameter<'a>) {
        oxc_ast_visit::walk::walk_catch_parameter(self, it);
    }

    fn visit_debugger_statement(&mut self, it: &oxc::ast::ast::DebuggerStatement) {
        oxc_ast_visit::walk::walk_debugger_statement(self, it);
    }

    fn visit_binding_pattern(&mut self, it: &oxc::ast::ast::BindingPattern<'a>) {
        oxc_ast_visit::walk::walk_binding_pattern(self, it);
    }

    fn visit_binding_pattern_kind(&mut self, it: &oxc::ast::ast::BindingPatternKind<'a>) {
        oxc_ast_visit::walk::walk_binding_pattern_kind(self, it);
    }

    fn visit_assignment_pattern(&mut self, it: &oxc::ast::ast::AssignmentPattern<'a>) {
        oxc_ast_visit::walk::walk_assignment_pattern(self, it);
    }

    fn visit_object_pattern(&mut self, it: &oxc::ast::ast::ObjectPattern<'a>) {
        oxc_ast_visit::walk::walk_object_pattern(self, it);
    }

    fn visit_binding_property(&mut self, it: &oxc::ast::ast::BindingProperty<'a>) {
        oxc_ast_visit::walk::walk_binding_property(self, it);
    }

    fn visit_array_pattern(&mut self, it: &oxc::ast::ast::ArrayPattern<'a>) {
        oxc_ast_visit::walk::walk_array_pattern(self, it);
    }

    fn visit_binding_rest_element(&mut self, it: &oxc::ast::ast::BindingRestElement<'a>) {
        oxc_ast_visit::walk::walk_binding_rest_element(self, it);
    }

    fn visit_function(
        &mut self,
        it: &oxc::ast::ast::Function<'a>,
        flags: oxc::semantic::ScopeFlags,
    ) {
        oxc_ast_visit::walk::walk_function(self, it, flags);
    }

    fn visit_formal_parameters(&mut self, it: &oxc::ast::ast::FormalParameters<'a>) {
        oxc_ast_visit::walk::walk_formal_parameters(self, it);
    }

    fn visit_formal_parameter(&mut self, it: &oxc::ast::ast::FormalParameter<'a>) {
        oxc_ast_visit::walk::walk_formal_parameter(self, it);
    }

    fn visit_function_body(&mut self, it: &oxc::ast::ast::FunctionBody<'a>) {
        oxc_ast_visit::walk::walk_function_body(self, it);
    }

    fn visit_arrow_function_expression(&mut self, it: &oxc::ast::ast::ArrowFunctionExpression<'a>) {
        oxc_ast_visit::walk::walk_arrow_function_expression(self, it);
    }

    fn visit_yield_expression(&mut self, it: &oxc::ast::ast::YieldExpression<'a>) {
        oxc_ast_visit::walk::walk_yield_expression(self, it);
    }

    fn visit_class(&mut self, it: &oxc::ast::ast::Class<'a>) {
        oxc_ast_visit::walk::walk_class(self, it);
    }

    fn visit_class_body(&mut self, it: &oxc::ast::ast::ClassBody<'a>) {
        oxc_ast_visit::walk::walk_class_body(self, it);
    }

    fn visit_class_element(&mut self, it: &oxc::ast::ast::ClassElement<'a>) {
        oxc_ast_visit::walk::walk_class_element(self, it);
    }

    fn visit_method_definition(&mut self, it: &oxc::ast::ast::MethodDefinition<'a>) {
        oxc_ast_visit::walk::walk_method_definition(self, it);
    }

    fn visit_property_definition(&mut self, it: &oxc::ast::ast::PropertyDefinition<'a>) {
        oxc_ast_visit::walk::walk_property_definition(self, it);
    }

    fn visit_static_block(&mut self, it: &oxc::ast::ast::StaticBlock<'a>) {
        oxc_ast_visit::walk::walk_static_block(self, it);
    }

    fn visit_module_declaration(&mut self, it: &oxc::ast::ast::ModuleDeclaration<'a>) {
        oxc_ast_visit::walk::walk_module_declaration(self, it);
    }

    fn visit_accessor_property(&mut self, it: &oxc::ast::ast::AccessorProperty<'a>) {
        oxc_ast_visit::walk::walk_accessor_property(self, it);
    }

    fn visit_import_expression(&mut self, it: &oxc::ast::ast::ImportExpression<'a>) {
        oxc_ast_visit::walk::walk_import_expression(self, it);
    }

    fn visit_import_declaration(&mut self, it: &oxc::ast::ast::ImportDeclaration<'a>) {
        oxc_ast_visit::walk::walk_import_declaration(self, it);
    }

    fn visit_import_declaration_specifier(
        &mut self,
        it: &oxc::ast::ast::ImportDeclarationSpecifier<'a>,
    ) {
        oxc_ast_visit::walk::walk_import_declaration_specifier(self, it);
    }

    fn visit_import_specifier(&mut self, it: &oxc::ast::ast::ImportSpecifier<'a>) {
        oxc_ast_visit::walk::walk_import_specifier(self, it);
    }

    fn visit_import_default_specifier(&mut self, it: &oxc::ast::ast::ImportDefaultSpecifier<'a>) {
        oxc_ast_visit::walk::walk_import_default_specifier(self, it);
    }

    fn visit_import_namespace_specifier(
        &mut self,
        it: &oxc::ast::ast::ImportNamespaceSpecifier<'a>,
    ) {
        oxc_ast_visit::walk::walk_import_namespace_specifier(self, it);
    }

    fn visit_with_clause(&mut self, it: &oxc::ast::ast::WithClause<'a>) {
        oxc_ast_visit::walk::walk_with_clause(self, it);
    }

    fn visit_import_attribute(&mut self, it: &oxc::ast::ast::ImportAttribute<'a>) {
        oxc_ast_visit::walk::walk_import_attribute(self, it);
    }

    fn visit_import_attribute_key(&mut self, it: &oxc::ast::ast::ImportAttributeKey<'a>) {
        oxc_ast_visit::walk::walk_import_attribute_key(self, it);
    }

    fn visit_export_named_declaration(&mut self, it: &oxc::ast::ast::ExportNamedDeclaration<'a>) {
        oxc_ast_visit::walk::walk_export_named_declaration(self, it);
    }

    fn visit_export_default_declaration(
        &mut self,
        it: &oxc::ast::ast::ExportDefaultDeclaration<'a>,
    ) {
        oxc_ast_visit::walk::walk_export_default_declaration(self, it);
    }

    fn visit_export_all_declaration(&mut self, it: &oxc::ast::ast::ExportAllDeclaration<'a>) {
        oxc_ast_visit::walk::walk_export_all_declaration(self, it);
    }

    fn visit_export_specifier(&mut self, it: &oxc::ast::ast::ExportSpecifier<'a>) {
        oxc_ast_visit::walk::walk_export_specifier(self, it);
    }

    fn visit_export_default_declaration_kind(
        &mut self,
        it: &oxc::ast::ast::ExportDefaultDeclarationKind<'a>,
    ) {
        oxc_ast_visit::walk::walk_export_default_declaration_kind(self, it);
    }

    fn visit_module_export_name(&mut self, it: &oxc::ast::ast::ModuleExportName<'a>) {
        oxc_ast_visit::walk::walk_module_export_name(self, it);
    }

    fn visit_v_8_intrinsic_expression(&mut self, it: &oxc::ast::ast::V8IntrinsicExpression<'a>) {
        oxc_ast_visit::walk::walk_v_8_intrinsic_expression(self, it);
    }

    fn visit_boolean_literal(&mut self, it: &oxc::ast::ast::BooleanLiteral) {
        oxc_ast_visit::walk::walk_boolean_literal(self, it);
    }

    fn visit_null_literal(&mut self, it: &oxc::ast::ast::NullLiteral) {
        oxc_ast_visit::walk::walk_null_literal(self, it);
    }

    fn visit_numeric_literal(&mut self, it: &oxc::ast::ast::NumericLiteral<'a>) {
        oxc_ast_visit::walk::walk_numeric_literal(self, it);
    }

    fn visit_string_literal(&mut self, it: &oxc::ast::ast::StringLiteral<'a>) {
        oxc_ast_visit::walk::walk_string_literal(self, it);
    }

    fn visit_big_int_literal(&mut self, it: &oxc::ast::ast::BigIntLiteral<'a>) {
        oxc_ast_visit::walk::walk_big_int_literal(self, it);
    }

    fn visit_reg_exp_literal(&mut self, it: &oxc::ast::ast::RegExpLiteral<'a>) {
        oxc_ast_visit::walk::walk_reg_exp_literal(self, it);
    }

    fn visit_jsx_element(&mut self, it: &oxc::ast::ast::JSXElement<'a>) {
        oxc_ast_visit::walk::walk_jsx_element(self, it);
    }

    fn visit_jsx_opening_element(&mut self, it: &oxc::ast::ast::JSXOpeningElement<'a>) {
        oxc_ast_visit::walk::walk_jsx_opening_element(self, it);
    }

    fn visit_jsx_closing_element(&mut self, it: &oxc::ast::ast::JSXClosingElement<'a>) {
        oxc_ast_visit::walk::walk_jsx_closing_element(self, it);
    }

    fn visit_jsx_fragment(&mut self, it: &oxc::ast::ast::JSXFragment<'a>) {
        oxc_ast_visit::walk::walk_jsx_fragment(self, it);
    }

    fn visit_jsx_opening_fragment(&mut self, it: &oxc::ast::ast::JSXOpeningFragment) {
        oxc_ast_visit::walk::walk_jsx_opening_fragment(self, it);
    }

    fn visit_jsx_closing_fragment(&mut self, it: &oxc::ast::ast::JSXClosingFragment) {
        oxc_ast_visit::walk::walk_jsx_closing_fragment(self, it);
    }

    fn visit_jsx_element_name(&mut self, it: &oxc::ast::ast::JSXElementName<'a>) {
        oxc_ast_visit::walk::walk_jsx_element_name(self, it);
    }

    fn visit_jsx_namespaced_name(&mut self, it: &oxc::ast::ast::JSXNamespacedName<'a>) {
        oxc_ast_visit::walk::walk_jsx_namespaced_name(self, it);
    }

    fn visit_jsx_member_expression(&mut self, it: &oxc::ast::ast::JSXMemberExpression<'a>) {
        oxc_ast_visit::walk::walk_jsx_member_expression(self, it);
    }

    fn visit_jsx_member_expression_object(
        &mut self,
        it: &oxc::ast::ast::JSXMemberExpressionObject<'a>,
    ) {
        oxc_ast_visit::walk::walk_jsx_member_expression_object(self, it);
    }

    fn visit_jsx_expression_container(&mut self, it: &oxc::ast::ast::JSXExpressionContainer<'a>) {
        oxc_ast_visit::walk::walk_jsx_expression_container(self, it);
    }

    fn visit_jsx_expression(&mut self, it: &oxc::ast::ast::JSXExpression<'a>) {
        oxc_ast_visit::walk::walk_jsx_expression(self, it);
    }

    fn visit_jsx_empty_expression(&mut self, it: &oxc::ast::ast::JSXEmptyExpression) {
        oxc_ast_visit::walk::walk_jsx_empty_expression(self, it);
    }

    fn visit_jsx_attribute_item(&mut self, it: &oxc::ast::ast::JSXAttributeItem<'a>) {
        oxc_ast_visit::walk::walk_jsx_attribute_item(self, it);
    }

    fn visit_jsx_attribute(&mut self, it: &oxc::ast::ast::JSXAttribute<'a>) {
        oxc_ast_visit::walk::walk_jsx_attribute(self, it);
    }

    fn visit_jsx_spread_attribute(&mut self, it: &oxc::ast::ast::JSXSpreadAttribute<'a>) {
        oxc_ast_visit::walk::walk_jsx_spread_attribute(self, it);
    }

    fn visit_jsx_attribute_name(&mut self, it: &oxc::ast::ast::JSXAttributeName<'a>) {
        oxc_ast_visit::walk::walk_jsx_attribute_name(self, it);
    }

    fn visit_jsx_attribute_value(&mut self, it: &oxc::ast::ast::JSXAttributeValue<'a>) {
        oxc_ast_visit::walk::walk_jsx_attribute_value(self, it);
    }

    fn visit_jsx_child(&mut self, it: &oxc::ast::ast::JSXChild<'a>) {
        oxc_ast_visit::walk::walk_jsx_child(self, it);
    }

    fn visit_jsx_spread_child(&mut self, it: &oxc::ast::ast::JSXSpreadChild<'a>) {
        oxc_ast_visit::walk::walk_jsx_spread_child(self, it);
    }

    fn visit_jsx_text(&mut self, it: &oxc::ast::ast::JSXText<'a>) {
        oxc_ast_visit::walk::walk_jsx_text(self, it);
    }

    fn visit_ts_this_parameter(&mut self, it: &oxc::ast::ast::TSThisParameter<'a>) {
        oxc_ast_visit::walk::walk_ts_this_parameter(self, it);
    }

    fn visit_ts_enum_declaration(&mut self, it: &oxc::ast::ast::TSEnumDeclaration<'a>) {
        oxc_ast_visit::walk::walk_ts_enum_declaration(self, it);
    }

    fn visit_ts_enum_body(&mut self, it: &oxc::ast::ast::TSEnumBody<'a>) {
        oxc_ast_visit::walk::walk_ts_enum_body(self, it);
    }

    fn visit_ts_enum_member(&mut self, it: &oxc::ast::ast::TSEnumMember<'a>) {
        oxc_ast_visit::walk::walk_ts_enum_member(self, it);
    }

    fn visit_ts_enum_member_name(&mut self, it: &oxc::ast::ast::TSEnumMemberName<'a>) {
        oxc_ast_visit::walk::walk_ts_enum_member_name(self, it);
    }

    fn visit_ts_type_annotation(&mut self, it: &oxc::ast::ast::TSTypeAnnotation<'a>) {
        oxc_ast_visit::walk::walk_ts_type_annotation(self, it);
    }

    fn visit_ts_literal_type(&mut self, it: &oxc::ast::ast::TSLiteralType<'a>) {
        oxc_ast_visit::walk::walk_ts_literal_type(self, it);
    }

    fn visit_ts_literal(&mut self, it: &oxc::ast::ast::TSLiteral<'a>) {
        oxc_ast_visit::walk::walk_ts_literal(self, it);
    }

    fn visit_ts_type(&mut self, it: &oxc::ast::ast::TSType<'a>) {
        oxc_ast_visit::walk::walk_ts_type(self, it);
    }

    fn visit_ts_conditional_type(&mut self, it: &oxc::ast::ast::TSConditionalType<'a>) {
        oxc_ast_visit::walk::walk_ts_conditional_type(self, it);
    }

    fn visit_ts_union_type(&mut self, it: &oxc::ast::ast::TSUnionType<'a>) {
        oxc_ast_visit::walk::walk_ts_union_type(self, it);
    }

    fn visit_ts_intersection_type(&mut self, it: &oxc::ast::ast::TSIntersectionType<'a>) {
        oxc_ast_visit::walk::walk_ts_intersection_type(self, it);
    }

    fn visit_ts_parenthesized_type(&mut self, it: &oxc::ast::ast::TSParenthesizedType<'a>) {
        oxc_ast_visit::walk::walk_ts_parenthesized_type(self, it);
    }

    fn visit_ts_type_operator(&mut self, it: &oxc::ast::ast::TSTypeOperator<'a>) {
        oxc_ast_visit::walk::walk_ts_type_operator(self, it);
    }

    fn visit_ts_array_type(&mut self, it: &oxc::ast::ast::TSArrayType<'a>) {
        oxc_ast_visit::walk::walk_ts_array_type(self, it);
    }

    fn visit_ts_indexed_access_type(&mut self, it: &oxc::ast::ast::TSIndexedAccessType<'a>) {
        oxc_ast_visit::walk::walk_ts_indexed_access_type(self, it);
    }

    fn visit_ts_tuple_type(&mut self, it: &oxc::ast::ast::TSTupleType<'a>) {
        oxc_ast_visit::walk::walk_ts_tuple_type(self, it);
    }

    fn visit_ts_named_tuple_member(&mut self, it: &oxc::ast::ast::TSNamedTupleMember<'a>) {
        oxc_ast_visit::walk::walk_ts_named_tuple_member(self, it);
    }

    fn visit_ts_optional_type(&mut self, it: &oxc::ast::ast::TSOptionalType<'a>) {
        oxc_ast_visit::walk::walk_ts_optional_type(self, it);
    }

    fn visit_ts_rest_type(&mut self, it: &oxc::ast::ast::TSRestType<'a>) {
        oxc_ast_visit::walk::walk_ts_rest_type(self, it);
    }

    fn visit_ts_tuple_element(&mut self, it: &oxc::ast::ast::TSTupleElement<'a>) {
        oxc_ast_visit::walk::walk_ts_tuple_element(self, it);
    }

    fn visit_ts_any_keyword(&mut self, it: &oxc::ast::ast::TSAnyKeyword) {
        oxc_ast_visit::walk::walk_ts_any_keyword(self, it);
    }

    fn visit_ts_string_keyword(&mut self, it: &oxc::ast::ast::TSStringKeyword) {
        oxc_ast_visit::walk::walk_ts_string_keyword(self, it);
    }

    fn visit_ts_boolean_keyword(&mut self, it: &oxc::ast::ast::TSBooleanKeyword) {
        oxc_ast_visit::walk::walk_ts_boolean_keyword(self, it);
    }

    fn visit_ts_number_keyword(&mut self, it: &oxc::ast::ast::TSNumberKeyword) {
        oxc_ast_visit::walk::walk_ts_number_keyword(self, it);
    }

    fn visit_ts_never_keyword(&mut self, it: &oxc::ast::ast::TSNeverKeyword) {
        oxc_ast_visit::walk::walk_ts_never_keyword(self, it);
    }

    fn visit_ts_intrinsic_keyword(&mut self, it: &oxc::ast::ast::TSIntrinsicKeyword) {
        oxc_ast_visit::walk::walk_ts_intrinsic_keyword(self, it);
    }

    fn visit_ts_unknown_keyword(&mut self, it: &oxc::ast::ast::TSUnknownKeyword) {
        oxc_ast_visit::walk::walk_ts_unknown_keyword(self, it);
    }

    fn visit_ts_null_keyword(&mut self, it: &oxc::ast::ast::TSNullKeyword) {
        oxc_ast_visit::walk::walk_ts_null_keyword(self, it);
    }

    fn visit_ts_undefined_keyword(&mut self, it: &oxc::ast::ast::TSUndefinedKeyword) {
        oxc_ast_visit::walk::walk_ts_undefined_keyword(self, it);
    }

    fn visit_ts_void_keyword(&mut self, it: &oxc::ast::ast::TSVoidKeyword) {
        oxc_ast_visit::walk::walk_ts_void_keyword(self, it);
    }

    fn visit_ts_symbol_keyword(&mut self, it: &oxc::ast::ast::TSSymbolKeyword) {
        oxc_ast_visit::walk::walk_ts_symbol_keyword(self, it);
    }

    fn visit_ts_this_type(&mut self, it: &oxc::ast::ast::TSThisType) {
        oxc_ast_visit::walk::walk_ts_this_type(self, it);
    }

    fn visit_ts_object_keyword(&mut self, it: &oxc::ast::ast::TSObjectKeyword) {
        oxc_ast_visit::walk::walk_ts_object_keyword(self, it);
    }

    fn visit_ts_big_int_keyword(&mut self, it: &oxc::ast::ast::TSBigIntKeyword) {
        oxc_ast_visit::walk::walk_ts_big_int_keyword(self, it);
    }

    fn visit_ts_type_reference(&mut self, it: &oxc::ast::ast::TSTypeReference<'a>) {
        oxc_ast_visit::walk::walk_ts_type_reference(self, it);
    }

    fn visit_ts_qualified_name(&mut self, it: &oxc::ast::ast::TSQualifiedName<'a>) {
        oxc_ast_visit::walk::walk_ts_qualified_name(self, it);
    }

    fn visit_ts_type_parameter_instantiation(
        &mut self,
        it: &oxc::ast::ast::TSTypeParameterInstantiation<'a>,
    ) {
        oxc_ast_visit::walk::walk_ts_type_parameter_instantiation(self, it);
    }

    fn visit_ts_type_parameter(&mut self, it: &oxc::ast::ast::TSTypeParameter<'a>) {
        oxc_ast_visit::walk::walk_ts_type_parameter(self, it);
    }

    fn visit_ts_type_parameter_declaration(
        &mut self,
        it: &oxc::ast::ast::TSTypeParameterDeclaration<'a>,
    ) {
        oxc_ast_visit::walk::walk_ts_type_parameter_declaration(self, it);
    }

    fn visit_ts_type_alias_declaration(&mut self, it: &oxc::ast::ast::TSTypeAliasDeclaration<'a>) {
        oxc_ast_visit::walk::walk_ts_type_alias_declaration(self, it);
    }

    fn visit_ts_class_implements(&mut self, it: &oxc::ast::ast::TSClassImplements<'a>) {
        oxc_ast_visit::walk::walk_ts_class_implements(self, it);
    }

    fn visit_ts_interface_declaration(&mut self, it: &oxc::ast::ast::TSInterfaceDeclaration<'a>) {
        oxc_ast_visit::walk::walk_ts_interface_declaration(self, it);
    }

    fn visit_ts_interface_body(&mut self, it: &oxc::ast::ast::TSInterfaceBody<'a>) {
        oxc_ast_visit::walk::walk_ts_interface_body(self, it);
    }

    fn visit_ts_property_signature(&mut self, it: &oxc::ast::ast::TSPropertySignature<'a>) {
        oxc_ast_visit::walk::walk_ts_property_signature(self, it);
    }

    fn visit_ts_signature(&mut self, it: &oxc::ast::ast::TSSignature<'a>) {
        oxc_ast_visit::walk::walk_ts_signature(self, it);
    }

    fn visit_ts_index_signature(&mut self, it: &oxc::ast::ast::TSIndexSignature<'a>) {
        oxc_ast_visit::walk::walk_ts_index_signature(self, it);
    }

    fn visit_ts_call_signature_declaration(
        &mut self,
        it: &oxc::ast::ast::TSCallSignatureDeclaration<'a>,
    ) {
        oxc_ast_visit::walk::walk_ts_call_signature_declaration(self, it);
    }

    fn visit_ts_method_signature(&mut self, it: &oxc::ast::ast::TSMethodSignature<'a>) {
        oxc_ast_visit::walk::walk_ts_method_signature(self, it);
    }

    fn visit_ts_construct_signature_declaration(
        &mut self,
        it: &oxc::ast::ast::TSConstructSignatureDeclaration<'a>,
    ) {
        oxc_ast_visit::walk::walk_ts_construct_signature_declaration(self, it);
    }

    fn visit_ts_index_signature_name(&mut self, it: &oxc::ast::ast::TSIndexSignatureName<'a>) {
        oxc_ast_visit::walk::walk_ts_index_signature_name(self, it);
    }

    fn visit_ts_interface_heritage(&mut self, it: &oxc::ast::ast::TSInterfaceHeritage<'a>) {
        oxc_ast_visit::walk::walk_ts_interface_heritage(self, it);
    }

    fn visit_ts_type_predicate(&mut self, it: &oxc::ast::ast::TSTypePredicate<'a>) {
        oxc_ast_visit::walk::walk_ts_type_predicate(self, it);
    }

    fn visit_ts_type_predicate_name(&mut self, it: &oxc::ast::ast::TSTypePredicateName<'a>) {
        oxc_ast_visit::walk::walk_ts_type_predicate_name(self, it);
    }

    fn visit_ts_module_declaration(&mut self, it: &oxc::ast::ast::TSModuleDeclaration<'a>) {
        oxc_ast_visit::walk::walk_ts_module_declaration(self, it);
    }

    fn visit_ts_module_declaration_name(
        &mut self,
        it: &oxc::ast::ast::TSModuleDeclarationName<'a>,
    ) {
        oxc_ast_visit::walk::walk_ts_module_declaration_name(self, it);
    }

    fn visit_ts_module_declaration_body(
        &mut self,
        it: &oxc::ast::ast::TSModuleDeclarationBody<'a>,
    ) {
        oxc_ast_visit::walk::walk_ts_module_declaration_body(self, it);
    }

    fn visit_ts_module_block(&mut self, it: &oxc::ast::ast::TSModuleBlock<'a>) {
        oxc_ast_visit::walk::walk_ts_module_block(self, it);
    }

    fn visit_ts_type_literal(&mut self, it: &oxc::ast::ast::TSTypeLiteral<'a>) {
        oxc_ast_visit::walk::walk_ts_type_literal(self, it);
    }

    fn visit_ts_infer_type(&mut self, it: &oxc::ast::ast::TSInferType<'a>) {
        oxc_ast_visit::walk::walk_ts_infer_type(self, it);
    }

    fn visit_ts_type_query(&mut self, it: &oxc::ast::ast::TSTypeQuery<'a>) {
        oxc_ast_visit::walk::walk_ts_type_query(self, it);
    }

    fn visit_ts_type_query_expr_name(&mut self, it: &oxc::ast::ast::TSTypeQueryExprName<'a>) {
        oxc_ast_visit::walk::walk_ts_type_query_expr_name(self, it);
    }

    fn visit_ts_import_type(&mut self, it: &oxc::ast::ast::TSImportType<'a>) {
        oxc_ast_visit::walk::walk_ts_import_type(self, it);
    }

    fn visit_ts_function_type(&mut self, it: &oxc::ast::ast::TSFunctionType<'a>) {
        oxc_ast_visit::walk::walk_ts_function_type(self, it);
    }

    fn visit_ts_constructor_type(&mut self, it: &oxc::ast::ast::TSConstructorType<'a>) {
        oxc_ast_visit::walk::walk_ts_constructor_type(self, it);
    }

    fn visit_ts_mapped_type(&mut self, it: &oxc::ast::ast::TSMappedType<'a>) {
        oxc_ast_visit::walk::walk_ts_mapped_type(self, it);
    }

    fn visit_ts_template_literal_type(&mut self, it: &oxc::ast::ast::TSTemplateLiteralType<'a>) {
        oxc_ast_visit::walk::walk_ts_template_literal_type(self, it);
    }

    fn visit_ts_as_expression(&mut self, it: &oxc::ast::ast::TSAsExpression<'a>) {
        oxc_ast_visit::walk::walk_ts_as_expression(self, it);
    }

    fn visit_ts_satisfies_expression(&mut self, it: &oxc::ast::ast::TSSatisfiesExpression<'a>) {
        oxc_ast_visit::walk::walk_ts_satisfies_expression(self, it);
    }

    fn visit_ts_type_assertion(&mut self, it: &oxc::ast::ast::TSTypeAssertion<'a>) {
        oxc_ast_visit::walk::walk_ts_type_assertion(self, it);
    }

    fn visit_ts_import_equals_declaration(
        &mut self,
        it: &oxc::ast::ast::TSImportEqualsDeclaration<'a>,
    ) {
        oxc_ast_visit::walk::walk_ts_import_equals_declaration(self, it);
    }

    fn visit_ts_module_reference(&mut self, it: &oxc::ast::ast::TSModuleReference<'a>) {
        oxc_ast_visit::walk::walk_ts_module_reference(self, it);
    }

    fn visit_ts_external_module_reference(
        &mut self,
        it: &oxc::ast::ast::TSExternalModuleReference<'a>,
    ) {
        oxc_ast_visit::walk::walk_ts_external_module_reference(self, it);
    }

    fn visit_ts_non_null_expression(&mut self, it: &oxc::ast::ast::TSNonNullExpression<'a>) {
        oxc_ast_visit::walk::walk_ts_non_null_expression(self, it);
    }

    fn visit_decorator(&mut self, it: &oxc::ast::ast::Decorator<'a>) {
        oxc_ast_visit::walk::walk_decorator(self, it);
    }

    fn visit_ts_export_assignment(&mut self, it: &oxc::ast::ast::TSExportAssignment<'a>) {
        oxc_ast_visit::walk::walk_ts_export_assignment(self, it);
    }

    fn visit_ts_namespace_export_declaration(
        &mut self,
        it: &oxc::ast::ast::TSNamespaceExportDeclaration<'a>,
    ) {
        oxc_ast_visit::walk::walk_ts_namespace_export_declaration(self, it);
    }

    fn visit_ts_instantiation_expression(
        &mut self,
        it: &oxc::ast::ast::TSInstantiationExpression<'a>,
    ) {
        oxc_ast_visit::walk::walk_ts_instantiation_expression(self, it);
    }

    fn visit_js_doc_nullable_type(&mut self, it: &oxc::ast::ast::JSDocNullableType<'a>) {
        oxc_ast_visit::walk::walk_js_doc_nullable_type(self, it);
    }

    fn visit_js_doc_non_nullable_type(&mut self, it: &oxc::ast::ast::JSDocNonNullableType<'a>) {
        oxc_ast_visit::walk::walk_js_doc_non_nullable_type(self, it);
    }

    fn visit_js_doc_unknown_type(&mut self, it: &oxc::ast::ast::JSDocUnknownType) {
        oxc_ast_visit::walk::walk_js_doc_unknown_type(self, it);
    }

    fn visit_span(&mut self, it: &Span) {
        oxc_ast_visit::walk::walk_span(self, it);
    }

    fn visit_directives(&mut self, it: &oxc::allocator::Vec<'a, oxc::ast::ast::Directive<'a>>) {
        oxc_ast_visit::walk::walk_directives(self, it);
    }

    fn visit_statements(&mut self, it: &oxc::allocator::Vec<'a, oxc::ast::ast::Statement<'a>>) {
        oxc_ast_visit::walk::walk_statements(self, it);
    }

    fn visit_array_expression_elements(
        &mut self,
        it: &oxc::allocator::Vec<'a, oxc::ast::ast::ArrayExpressionElement<'a>>,
    ) {
        oxc_ast_visit::walk::walk_array_expression_elements(self, it);
    }

    fn visit_object_property_kinds(
        &mut self,
        it: &oxc::allocator::Vec<'a, oxc::ast::ast::ObjectPropertyKind<'a>>,
    ) {
        oxc_ast_visit::walk::walk_object_property_kinds(self, it);
    }

    fn visit_template_elements(
        &mut self,
        it: &oxc::allocator::Vec<'a, oxc::ast::ast::TemplateElement<'a>>,
    ) {
        oxc_ast_visit::walk::walk_template_elements(self, it);
    }

    fn visit_expressions(&mut self, it: &oxc::allocator::Vec<'a, oxc::ast::ast::Expression<'a>>) {
        oxc_ast_visit::walk::walk_expressions(self, it);
    }

    fn visit_arguments(&mut self, it: &oxc::allocator::Vec<'a, oxc::ast::ast::Argument<'a>>) {
        oxc_ast_visit::walk::walk_arguments(self, it);
    }

    fn visit_assignment_target_properties(
        &mut self,
        it: &oxc::allocator::Vec<'a, oxc::ast::ast::AssignmentTargetProperty<'a>>,
    ) {
        oxc_ast_visit::walk::walk_assignment_target_properties(self, it);
    }

    fn visit_variable_declarators(
        &mut self,
        it: &oxc::allocator::Vec<'a, oxc::ast::ast::VariableDeclarator<'a>>,
    ) {
        oxc_ast_visit::walk::walk_variable_declarators(self, it);
    }

    fn visit_switch_cases(&mut self, it: &oxc::allocator::Vec<'a, oxc::ast::ast::SwitchCase<'a>>) {
        oxc_ast_visit::walk::walk_switch_cases(self, it);
    }

    fn visit_binding_properties(
        &mut self,
        it: &oxc::allocator::Vec<'a, oxc::ast::ast::BindingProperty<'a>>,
    ) {
        oxc_ast_visit::walk::walk_binding_properties(self, it);
    }

    fn visit_formal_parameter_list(
        &mut self,
        it: &oxc::allocator::Vec<'a, oxc::ast::ast::FormalParameter<'a>>,
    ) {
        oxc_ast_visit::walk::walk_formal_parameter_list(self, it);
    }

    fn visit_decorators(&mut self, it: &oxc::allocator::Vec<'a, oxc::ast::ast::Decorator<'a>>) {
        oxc_ast_visit::walk::walk_decorators(self, it);
    }

    fn visit_ts_class_implements_list(
        &mut self,
        it: &oxc::allocator::Vec<'a, oxc::ast::ast::TSClassImplements<'a>>,
    ) {
        oxc_ast_visit::walk::walk_ts_class_implements_list(self, it);
    }

    fn visit_class_elements(
        &mut self,
        it: &oxc::allocator::Vec<'a, oxc::ast::ast::ClassElement<'a>>,
    ) {
        oxc_ast_visit::walk::walk_class_elements(self, it);
    }

    fn visit_import_declaration_specifiers(
        &mut self,
        it: &oxc::allocator::Vec<'a, oxc::ast::ast::ImportDeclarationSpecifier<'a>>,
    ) {
        oxc_ast_visit::walk::walk_import_declaration_specifiers(self, it);
    }

    fn visit_import_attributes(
        &mut self,
        it: &oxc::allocator::Vec<'a, oxc::ast::ast::ImportAttribute<'a>>,
    ) {
        oxc_ast_visit::walk::walk_import_attributes(self, it);
    }

    fn visit_export_specifiers(
        &mut self,
        it: &oxc::allocator::Vec<'a, oxc::ast::ast::ExportSpecifier<'a>>,
    ) {
        oxc_ast_visit::walk::walk_export_specifiers(self, it);
    }

    fn visit_jsx_children(&mut self, it: &oxc::allocator::Vec<'a, oxc::ast::ast::JSXChild<'a>>) {
        oxc_ast_visit::walk::walk_jsx_children(self, it);
    }

    fn visit_jsx_attribute_items(
        &mut self,
        it: &oxc::allocator::Vec<'a, oxc::ast::ast::JSXAttributeItem<'a>>,
    ) {
        oxc_ast_visit::walk::walk_jsx_attribute_items(self, it);
    }

    fn visit_ts_enum_members(
        &mut self,
        it: &oxc::allocator::Vec<'a, oxc::ast::ast::TSEnumMember<'a>>,
    ) {
        oxc_ast_visit::walk::walk_ts_enum_members(self, it);
    }

    fn visit_ts_types(&mut self, it: &oxc::allocator::Vec<'a, oxc::ast::ast::TSType<'a>>) {
        oxc_ast_visit::walk::walk_ts_types(self, it);
    }

    fn visit_ts_tuple_elements(
        &mut self,
        it: &oxc::allocator::Vec<'a, oxc::ast::ast::TSTupleElement<'a>>,
    ) {
        oxc_ast_visit::walk::walk_ts_tuple_elements(self, it);
    }

    fn visit_ts_type_parameters(
        &mut self,
        it: &oxc::allocator::Vec<'a, oxc::ast::ast::TSTypeParameter<'a>>,
    ) {
        oxc_ast_visit::walk::walk_ts_type_parameters(self, it);
    }

    fn visit_ts_interface_heritages(
        &mut self,
        it: &oxc::allocator::Vec<'a, oxc::ast::ast::TSInterfaceHeritage<'a>>,
    ) {
        oxc_ast_visit::walk::walk_ts_interface_heritages(self, it);
    }

    fn visit_ts_signatures(
        &mut self,
        it: &oxc::allocator::Vec<'a, oxc::ast::ast::TSSignature<'a>>,
    ) {
        oxc_ast_visit::walk::walk_ts_signatures(self, it);
    }

    fn visit_ts_index_signature_names(
        &mut self,
        it: &oxc::allocator::Vec<'a, oxc::ast::ast::TSIndexSignatureName<'a>>,
    ) {
        oxc_ast_visit::walk::walk_ts_index_signature_names(self, it);
    }

    fn visit_spans(&mut self, it: &oxc::allocator::Vec<'a, Span>) {
        oxc_ast_visit::walk::walk_spans(self, it);
    }
}
