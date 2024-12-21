use super::*;

#[test]
fn test_simple_derive_struct() {
    let derive = syn::parse_quote! {
        struct TestStruct{
            field1:i32,
            field2:i32,
        }
    };

    let trait_item = syn::parse_quote! {
        trait TestTrait{
            fn test(&self,__a:i32,__b:i32);
        }
    };

    let result = simple_derive(&derive, &trait_item, None);
    assert_eq!(
        result.to_string(),
        quote! {
            impl TestTrait for TestStruct {
                fn test(&self, __a: i32, __b: i32) {
                    TestTrait::test(&self.field1, __a, __b);
                    TestTrait::test(&self.field2, __a, __b);
                }
            }
        }
        .to_string()
    );
}

#[test]
fn test_simple_derive_enum() {
    let derive = syn::parse_quote! {
        enum TestEnum{
            A(i32),
            B(u32),
        }
    };

    let trait_item = syn::parse_quote! {
        trait TestTrait{
            fn test(&self,__a:i32,__b:i32);
        }
    };

    let result = simple_derive(&derive, &trait_item, None);
    assert_eq!(
        result.to_string(),
        quote! {
            impl TestTrait for TestEnum {
                fn test(&self, __a: i32, __b: i32) {
                    match self {
                        Self::A(__a0) => TestTrait::test(__a0, __a, __b),
                        Self::B(__a1) => TestTrait::test(__a1, __a, __b),
                    }
                }
            }
        }
        .to_string()
    );
}
