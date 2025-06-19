# TypeScript test runner

Experimental runner for TypeScript compiler and conformance tests:

- Docs: [TypeScript-Compiler-Notes/systems/testing/baselines.md](https://github.com/microsoft/TypeScript-Compiler-Notes/blob/main/systems/testing/baselines.md)
- Tests: [TypeScript/tests/baselines/reference](https://github.com/microsoft/TypeScript/tree/main/tests/baselines/reference)

It was used to explore feasibility of: https://github.com/oxc-project/oxc/pull/2912

> [!NOTE]
>
> This project already implemented the key features but it is far from usable. See [Status](#status).

## How to run this?

```sh
cargo run --bin test-runner /path/to/TypeScript/repo
```

![demo](https://github.com/user-attachments/assets/d188ad77-8d09-4c48-b4ba-579e51dbfffc)
<sub>Tested with: [TypeScript#56a08250f3516b3f5bc120d6c7ab4450a9a69352](https://github.com/microsoft/TypeScript/tree/56a08250f3516b3f5bc120d6c7ab4450a9a69352)</sub>

## Status

- [x] Parse all compiler (`tests/cases/compiler`) and conformance (`tests/cases/conformance`) tests
  - These are all the tests that are used to test TypeScript type checker
  - A test consists of tsconfig settings and a set of source files.
- [x] Parse error baselines
  - These are used to make sure invalid code constructs and types throw errors as expected
- [x] Parse type baselines
  - These contain type annotations for every node in the AST and are used to verify types inferred by the type checker
- [x] Parse individual files within each test with OXC
- [ ] Resolve imports using oxc_resolver with a virtual file system (only containing files defined in the test)
- [ ] Implement OXC Visitor to walk the tree in the same order as TSC
  - Needs to mimic the logic of `TypeWriterWalker`: [src/harness/typeWriter.ts#L179](https://github.com/microsoft/TypeScript/blob/479285d0ac293c35a926508d17f0bb5eca7e0303/src/harness/typeWriter.ts#L179)
- [ ] Assert types and errors match the baselines
