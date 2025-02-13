use std::{borrow::BorrowMut, collections::HashMap, hash::Hash, ops::Deref};

use crate::parser::{self, BaseType, Def, Node, Parser, ParserResult, Struct};

#[derive(Debug)]
pub struct SemanticAnalyzer {
    // pub parser_result: ParserResult,
    pub diagnostics: Diagnostics,
}

#[derive(Debug)]
pub struct Diagnostics {}

impl SemanticAnalyzer {
    pub fn run(result: &mut ParserResult) -> SemanticAnalyzer {
        Self::transform_ast(result)
    }

    pub fn transform_ast(result: &mut ParserResult) -> SemanticAnalyzer {
        let mut attribute_index = HashMap::new();
        let mut method_index = HashMap::new();

        match &mut result.module {
            Node::Module(module) => {
                populate_class_index(&result.index.class_index, &mut attribute_index);
                populate_method_index(module, &mut method_index);
                run_type_inference(
                    module,
                    method_index,
                    attribute_index,
                    &result.index.struct_index,
                );
            }
            _ => todo!(),
        }

        SemanticAnalyzer {
            diagnostics: Diagnostics {},
        }
    }
}

fn populate_class_index(
    class_index: &HashMap<String, parser::Class>,
    attribute_index: &mut HashMap<String, (i32, BaseType)>,
) {
    class_index.values().for_each(|class| {
        for attribute in &class.attributes {
            attribute_index.insert(
                format!("{}.{}", class.name, attribute.name),
                (attribute.index, attribute.return_type.clone()),
            );
        }
    });
}

fn populate_method_index(
    module: &mut crate::parser::Module,
    method_index: &mut HashMap<String, Option<BaseType>>,
) {
    module.methods.iter_mut().for_each(|node| match node {
        Node::Def(def_node) => {
            method_index.insert(
                def_node.prototype.name.clone(),
                def_node.prototype.return_type.clone(),
            );
        }
        Node::DefE(def_e_node) => {
            method_index.insert(
                def_e_node.prototype.name.clone(),
                def_e_node.prototype.return_type.clone(),
            );
        }
        _ => {}
    });
}

fn run_type_inference(
    module: &mut crate::parser::Module,
    mut method_index: HashMap<String, Option<BaseType>>,
    mut attribute_index: HashMap<String, (i32, BaseType)>,
    struct_index: &HashMap<String, parser::Struct>,
) {
    module.methods.iter_mut().for_each(|node| {
        match node {
            Node::Def(def_node) => {
                let mut lvar_index = HashMap::new();

                def_node.prototype.args.iter().for_each(|arg| {
                    lvar_index.insert(arg.name.clone(), Some(arg.return_type.clone()));
                });

                def_node.body.iter_mut().for_each(|node| match node {
                    Node::Access(access_node) => {
                        visit_access_node(&attribute_index, &lvar_index, access_node);
                    }
                    Node::AssignLocalVar(assignlocalvar_node) => {
                        let return_type = match assignlocalvar_node.value.as_mut() {
                            Node::Binary(binary_node) => visit_binary_node(
                                &attribute_index,
                                &method_index,
                                &lvar_index,
                                binary_node,
                            ),
                            Node::Call(call_node) => visit_call_node(
                                &attribute_index,
                                &method_index,
                                &lvar_index,
                                call_node,
                            ),
                            Node::Send(send_node) => visit_send_node(
                                &attribute_index,
                                &method_index,
                                &lvar_index,
                                send_node,
                            ),
                            Node::Access(access_node) => {
                                visit_access_node(&attribute_index, &lvar_index, access_node)
                            }
                            Node::AssignAttribute(_) => todo!(),
                            Node::AssignAttributeAccess(_) => todo!(),
                            Node::AssignLocalVar(_) => todo!(),
                            Node::Attribute(_) => todo!(),
                            Node::Class(_) => todo!(),
                            Node::Const(_) => todo!(),
                            Node::Def(_) => todo!(),
                            Node::DefE(_) => todo!(),
                            Node::Impl(_) => todo!(),
                            Node::Int(_) => Some(BaseType::Int),
                            Node::LocalVar(_) => todo!(),
                            Node::Loop(_) => todo!(),
                            Node::Module(_) => todo!(),
                            Node::Ret(_) => todo!(),
                            Node::SelfRef(_) => todo!(),
                            Node::StringLiteral(_) => Some(BaseType::Class("Str".to_string())),
                            Node::Trait(_) => todo!(),
                            Node::AssignConstant(_) => todo!(),
                            Node::Array(array) => {
                                array.items.iter_mut().for_each(|node| {
                                    match node {
                                        Node::Access(access_node) => visit_access_node(
                                            &attribute_index,
                                            &lvar_index,
                                            access_node,
                                        ),
                                        Node::Binary(node) => visit_binary_node(
                                            &attribute_index,
                                            &method_index,
                                            &lvar_index,
                                            node,
                                        ),
                                        Node::Call(node) => visit_call_node(
                                            &attribute_index,
                                            &method_index,
                                            &lvar_index,
                                            node,
                                        ),
                                        Node::Send(node) => visit_send_node(
                                            &attribute_index,
                                            &method_index,
                                            &lvar_index,
                                            node,
                                        ),
                                        Node::Int(node) => Some(BaseType::Int),
                                        _ => todo!(),
                                    };
                                });

                                Some(BaseType::Array(
                                    array.length,
                                    Box::new(array.item_type.clone()),
                                ))
                            }
                            Node::BuildStruct(struct_node) => visit_build_struct_node(
                                &attribute_index,
                                &method_index,
                                &lvar_index,
                                struct_node,
                                struct_index,
                            ),
                            Node::Struct(_) => todo!(),
                            Node::FnRef(_) => todo!(),
                        };

                        lvar_index.insert(assignlocalvar_node.name.clone(), return_type);
                    }
                    Node::Binary(binary_node) => {
                        visit_binary_node(
                            &attribute_index,
                            &method_index,
                            &lvar_index,
                            binary_node,
                        );
                    }
                    Node::Call(call_node) => {
                        visit_call_node(&attribute_index, &method_index, &lvar_index, call_node);
                    }
                    Node::Send(send_node) => {
                        visit_send_node(&attribute_index, &method_index, &lvar_index, send_node);
                    }
                    Node::Ret(ret_node) => {
                        visit_ret_node(&attribute_index, &method_index, &lvar_index, ret_node);
                    }
                    Node::AssignConstant(_) => todo!(),
                    Node::Attribute(_) => todo!(),
                    Node::Class(_) => todo!(),
                    Node::Def(_) => todo!(),
                    Node::DefE(_) => todo!(),
                    Node::Impl(_) => todo!(),
                    Node::Int(_) => {},
                    Node::StringLiteral(_) => {},
                    Node::LocalVar(node) => {
                        match node.return_type {
                            Some(_) => {}
                            None => todo!(),
                        }
                        // println!("{:#?}", node);
                    }
                    Node::Module(_) => todo!(),
                    Node::SelfRef(_) => todo!(),
                    Node::Trait(_) => todo!(),
                    Node::AssignAttribute(assign_attr_node) => {
                        match assign_attr_node.value.as_mut() {
                            Node::Binary(binary_node) => visit_binary_node(
                                &attribute_index,
                                &method_index,
                                &lvar_index,
                                binary_node,
                            ),
                            Node::Call(call_node) => visit_call_node(
                                &attribute_index,
                                &method_index,
                                &lvar_index,
                                call_node,
                            ),
                            Node::Send(send_node) => visit_send_node(
                                &attribute_index,
                                &method_index,
                                &lvar_index,
                                send_node,
                            ),
                            Node::Access(access_node) => {
                                visit_access_node(&attribute_index, &lvar_index, access_node)
                            }
                            Node::AssignAttribute(_) => todo!(),
                            Node::AssignAttributeAccess(_) => todo!(),
                            Node::AssignLocalVar(_) => todo!(),
                            Node::Attribute(_) => todo!(),
                            Node::Class(_) => todo!(),
                            Node::Const(_) => todo!(),
                            Node::Def(_) => todo!(),
                            Node::DefE(_) => todo!(),
                            Node::Impl(_) => todo!(),
                            Node::Int(_) => todo!(),
                            Node::LocalVar(lvar) => match lvar.return_type {
                                Some(_) => lvar.return_type.clone(),
                                None => todo!(),
                            },
                            Node::Loop(_) => todo!(),
                            Node::Module(_) => todo!(),
                            Node::Ret(_) => todo!(),
                            Node::SelfRef(_) => todo!(),
                            Node::StringLiteral(_) => todo!(),
                            Node::Trait(_) => todo!(),
                            Node::AssignConstant(_) => todo!(),
                            Node::Array(array) => {
                                array.items.iter_mut().for_each(|node| {
                                    match node {
                                        Node::Access(access_node) => visit_access_node(
                                            &attribute_index,
                                            &lvar_index,
                                            access_node,
                                        ),
                                        Node::Binary(node) => visit_binary_node(
                                            &attribute_index,
                                            &method_index,
                                            &lvar_index,
                                            node,
                                        ),
                                        Node::Call(node) => visit_call_node(
                                            &attribute_index,
                                            &method_index,
                                            &lvar_index,
                                            node,
                                        ),
                                        Node::Send(node) => visit_send_node(
                                            &attribute_index,
                                            &method_index,
                                            &lvar_index,
                                            node,
                                        ),
                                        _ => todo!(),
                                    };
                                });

                                Some(BaseType::Array(
                                    array.length,
                                    Box::new(array.item_type.clone()),
                                ))
                            }
                            Node::BuildStruct(_) => todo!(),
                            Node::Struct(_) => todo!(),
                            Node::FnRef(_) => todo!(),
                        };
                    }
                    Node::Const(_) => todo!(),
                    Node::AssignAttributeAccess(node) => {
                        visit_access_node(&attribute_index, &lvar_index, &mut node.access);

                        match node.value.as_mut() {
                            Node::Binary(binary_node) => visit_binary_node(
                                &attribute_index,
                                &method_index,
                                &lvar_index,
                                binary_node,
                            ),
                            Node::Call(call_node) => visit_call_node(
                                &attribute_index,
                                &method_index,
                                &lvar_index,
                                call_node,
                            ),
                            Node::Send(send_node) => visit_send_node(
                                &attribute_index,
                                &method_index,
                                &lvar_index,
                                send_node,
                            ),
                            Node::Access(access_node) => {
                                visit_access_node(&attribute_index, &lvar_index, access_node)
                            }
                            Node::AssignAttribute(_) => todo!(),
                            Node::AssignAttributeAccess(_) => todo!(),
                            Node::AssignLocalVar(_) => todo!(),
                            Node::Attribute(_) => todo!(),
                            Node::Class(_) => todo!(),
                            Node::Const(_) => todo!(),
                            Node::Def(_) => todo!(),
                            Node::DefE(_) => todo!(),
                            Node::Impl(_) => todo!(),
                            Node::Int(_) => todo!(),
                            Node::LocalVar(lvar) => match lvar.return_type {
                                Some(_) => lvar.return_type.clone(),
                                None => todo!(),
                            },
                            Node::Loop(_) => todo!(),
                            Node::Module(_) => todo!(),
                            Node::Ret(_) => todo!(),
                            Node::SelfRef(_) => todo!(),
                            Node::StringLiteral(_) => Some(BaseType::Class("Str".to_string())),
                            Node::Trait(_) => todo!(),
                            Node::AssignConstant(_) => todo!(),
                            Node::Array(array) => {
                                array.items.iter_mut().for_each(|node| {
                                    match node {
                                        Node::Access(access_node) => visit_access_node(
                                            &attribute_index,
                                            &lvar_index,
                                            access_node,
                                        ),
                                        Node::Binary(node) => visit_binary_node(
                                            &attribute_index,
                                            &method_index,
                                            &lvar_index,
                                            node,
                                        ),
                                        Node::Call(node) => visit_call_node(
                                            &attribute_index,
                                            &method_index,
                                            &lvar_index,
                                            node,
                                        ),
                                        Node::Send(node) => visit_send_node(
                                            &attribute_index,
                                            &method_index,
                                            &lvar_index,
                                            node,
                                        ),
                                        _ => todo!(),
                                    };
                                });

                                Some(BaseType::Array(
                                    array.length,
                                    Box::new(array.item_type.clone()),
                                ))
                            }
                            Node::BuildStruct(_) => todo!(),
                            Node::Struct(_) => todo!(),
                            Node::FnRef(_) => todo!(),
                        };
                    }
                    Node::Loop(loop_node) => {
                        loop_node.body.iter_mut().for_each(|node| {
                            match node {
                                Node::Access(access_node) => {
                                    visit_access_node(&attribute_index, &lvar_index, access_node)
                                }
                                Node::Binary(node) => visit_binary_node(
                                    &attribute_index,
                                    &method_index,
                                    &lvar_index,
                                    node,
                                ),
                                Node::Call(node) => visit_call_node(
                                    &attribute_index,
                                    &method_index,
                                    &lvar_index,
                                    node,
                                ),
                                Node::Send(node) => visit_send_node(
                                    &attribute_index,
                                    &method_index,
                                    &lvar_index,
                                    node,
                                ),
                                _ => todo!(),
                            };
                        });
                    }
                    Node::Array(array) => {
                        array.items.iter_mut().for_each(|node| {
                            match node {
                                Node::Access(access_node) => {
                                    visit_access_node(&attribute_index, &lvar_index, access_node)
                                }
                                Node::Binary(node) => visit_binary_node(
                                    &attribute_index,
                                    &method_index,
                                    &lvar_index,
                                    node,
                                ),
                                Node::Call(node) => visit_call_node(
                                    &attribute_index,
                                    &method_index,
                                    &lvar_index,
                                    node,
                                ),
                                Node::Send(node) => visit_send_node(
                                    &attribute_index,
                                    &method_index,
                                    &lvar_index,
                                    node,
                                ),
                                _ => todo!(),
                            };
                        });
                    }
                    Node::BuildStruct(_) => todo!(),
                    Node::Struct(_) => todo!(),
                    Node::FnRef(_) => todo!(),
                })
            }
            _ => {}
        };
    });
}

fn visit_ret_node(
    attribute_index: &HashMap<String, (i32, BaseType)>,
    method_index: &HashMap<String, Option<BaseType>>,
    lvar_index: &HashMap<String, Option<BaseType>>,
    ret_node: &mut crate::parser::Ret,
) {
    match ret_node.value.as_mut() {
        Node::Access(access_node) => visit_access_node(attribute_index, lvar_index, access_node),
        Node::Binary(node) => visit_binary_node(attribute_index, method_index, lvar_index, node),
        Node::Call(node) => visit_call_node(attribute_index, method_index, lvar_index, node),
        Node::Send(node) => visit_send_node(attribute_index, method_index, lvar_index, node),
        _ => todo!(),
    };
}

fn visit_access_node(
    attribute_index: &HashMap<String, (i32, BaseType)>,
    lvar_index: &HashMap<String, Option<BaseType>>,
    access_node: &mut crate::parser::Access,
) -> Option<BaseType> {
    let class_name = match access_node.receiver.as_mut() {
        Node::LocalVar(lvar) => {
            match lvar.return_type {
                Some(_) => {}
                None => todo!(),
            }

            let latest_return_type = lvar_index.get(&lvar.name).unwrap();
            lvar.return_type = latest_return_type.clone();

            pajama_class_name(&lvar.return_type.as_ref().unwrap())
        }
        Node::Access(_) => todo!(),
        Node::AssignAttribute(_) => todo!(),
        Node::AssignAttributeAccess(_) => todo!(),
        Node::AssignLocalVar(_) => todo!(),
        Node::Attribute(_) => todo!(),
        Node::Binary(_) => todo!(),
        Node::Call(_) => todo!(),
        Node::Class(_) => todo!(),
        Node::Const(_) => todo!(),
        Node::Def(_) => todo!(),
        Node::DefE(_) => todo!(),
        Node::Impl(_) => todo!(),
        Node::Int(_) => todo!(),
        Node::Loop(_) => todo!(),
        Node::Module(_) => todo!(),
        Node::Ret(_) => todo!(),
        Node::SelfRef(self_ref) => pajama_class_name(&self_ref.return_type),
        Node::Send(_) => todo!(),
        Node::StringLiteral(_) => todo!(),
        Node::Trait(_) => todo!(),
        Node::AssignConstant(_) => todo!(),
        Node::Array(_) => "Array".to_string(),
        Node::BuildStruct(_) => todo!(),
        Node::Struct(_) => todo!(),
        Node::FnRef(_) => todo!(),
    };

    let attribute_name = match access_node.message.as_mut() {
        Node::Attribute(attr_node) => attr_node.name.clone(),
        _ => todo!(),
    };

    let attr_key = format!("{}.{}", class_name, attribute_name);
    let (index, return_type) = attribute_index.get(&attr_key).unwrap();

    access_node.index = *index;
    access_node.return_type = Some(return_type.clone());

    Some(return_type.clone())
}

fn visit_binary_node(
    attribute_index: &HashMap<String, (i32, BaseType)>,
    method_index: &HashMap<String, Option<BaseType>>,
    lvar_index: &HashMap<String, Option<BaseType>>,
    binary_node: &mut crate::parser::Binary,
) -> Option<BaseType> {
    match binary_node.left.as_mut() {
        Node::Access(access_node) => visit_access_node(attribute_index, lvar_index, access_node),
        Node::Binary(node) => visit_binary_node(attribute_index, method_index, lvar_index, node),
        Node::Call(node) => visit_call_node(attribute_index, method_index, lvar_index, node),
        Node::Send(node) => visit_send_node(attribute_index, method_index, lvar_index, node),
        Node::Int(_) => Some(BaseType::Int),
        // _ => todo!(),
        _ => {
            todo!()
        },
    };

    match binary_node.right.as_mut() {
        Node::Access(access_node) => visit_access_node(attribute_index, lvar_index, access_node),
        Node::Binary(node) => visit_binary_node(attribute_index, method_index, lvar_index, node),
        Node::Call(node) => visit_call_node(attribute_index, method_index, lvar_index, node),
        Node::Send(node) => visit_send_node(attribute_index, method_index, lvar_index, node),
        Node::Int(_) => Some(BaseType::Int),
        _ => todo!(),
    }

    // todo maybe validate the operator here since now both the left and right
    // types are known
}

fn visit_call_node(
    attribute_index: &HashMap<String, (i32, BaseType)>,
    method_index: &HashMap<String, Option<BaseType>>,
    lvar_index: &HashMap<String, Option<BaseType>>,
    call_node: &mut crate::parser::Call,
) -> Option<BaseType> {
    println!("{:#?}", &call_node.fn_name);
    println!("{:#?}", method_index);

    let base_type = method_index.get(&call_node.fn_name).unwrap();
    call_node.return_type = base_type.clone();

    for arg in &mut call_node.args {
        println!("{:#?}", arg);

        match arg {
            Node::Access(access_node) => {
                visit_access_node(attribute_index, lvar_index, access_node);
            }
            Node::Call(node) => {
                visit_call_node(attribute_index, method_index, lvar_index, node);
            }
            Node::Send(node) => {
                visit_send_node(attribute_index, &method_index, lvar_index, node);
            }
            Node::Binary(node) => {
                visit_binary_node(attribute_index, method_index, lvar_index, node);
            }
            Node::LocalVar(lvar) => {
                match lvar.return_type {
                    Some(_) => {}
                    None => todo!(),
                }

                let latest_return_type = lvar_index.get(&lvar.name).unwrap();
                lvar.return_type = latest_return_type.clone();
            }
            Node::StringLiteral(_) => {}
            Node::Const(_) => {}
            Node::Int(_) => {}
            Node::SelfRef(self_ref) => {
                // Node::SelfRef(self_ref) => pajama_class_name(&self_ref.return_type),
            }
            _ => {
                println!("{:#?}", arg);
                todo!()
            }
        };
    }

    base_type.clone()
}

fn visit_send_node(
    attribute_index: &HashMap<String, (i32, BaseType)>,
    method_index: &HashMap<String, Option<BaseType>>,
    lvar_index: &HashMap<String, Option<BaseType>>,
    send_node: &mut crate::parser::Send,
) -> Option<BaseType> {
    let fn_name = match send_node.message.as_mut() {
        Node::Call(node) => &node.fn_name,
        _ => todo!(),
    };

    let basetype = match send_node.receiver.as_mut() {
        Node::Access(access_node) => visit_access_node(attribute_index, lvar_index, access_node),
        Node::Call(node) => visit_call_node(attribute_index, method_index, lvar_index, node),
        Node::Send(node) => visit_send_node(attribute_index, &method_index, lvar_index, node),
        Node::Binary(node) => visit_binary_node(attribute_index, method_index, lvar_index, node),
        Node::LocalVar(lvar) => {
            match lvar.return_type {
                Some(_) => {}
                None => {
                    // maybe a function ref!
                    if method_index.contains_key(&lvar.name) {
                        send_node.return_type = Some(BaseType::FnRef);
                        lvar.return_type = Some(BaseType::FnRef);
                        return lvar.return_type.clone();
                    } else {
                        // not found, return an error
                        todo!()
                    }
                }
            }

            let latest_return_type = lvar_index.get(&lvar.name).unwrap();
            lvar.return_type = latest_return_type.clone();
            latest_return_type.clone()
        }
        Node::Const(node) => {
            if fn_name == "new" || fn_name == "alloca" {
                send_node.return_type = Some(BaseType::Class(node.name.clone()));
                Some(BaseType::Class(node.name.clone()))
                // return;
            } else {
                todo!("class methods")
            }
        }
        _ => None,
    };

    let class_name = pajama_class_name(&basetype.as_ref().unwrap());
    let message_name = match send_node.message.as_mut() {
        Node::Call(node) => {
            let prefixed_name = format!("{}.{}", class_name, &node.fn_name);
            node.fn_name = prefixed_name.clone();

            visit_call_node(attribute_index, method_index, lvar_index, node);

            prefixed_name
        }
        _ => "".to_string(),
    };

    let base_type = method_index.get(&message_name).unwrap();
    match base_type {
        Some(bt) => {
            send_node.return_type = Some(bt.clone());
            Some(bt.clone())
        }
        None => None,
    }
}

fn visit_build_struct_node(
    attribute_index: &HashMap<String, (i32, BaseType)>,
    method_index: &HashMap<String, Option<BaseType>>,
    lvar_index: &HashMap<String, Option<BaseType>>,
    build_struct_node: &mut crate::parser::BuildStruct,
    struct_index: &HashMap<String, parser::Struct>,
) -> Option<BaseType> {
    build_struct_node.args.iter_mut().for_each(|node| {
        match node {
            Node::Access(access_node) => {
                visit_access_node(&attribute_index, &lvar_index, access_node);
            }
            Node::Binary(node) => {
                visit_binary_node(&attribute_index, &method_index, &lvar_index, node);
            }
            Node::Call(node) => {
                visit_call_node(&attribute_index, &method_index, &lvar_index, node);
            }
            Node::Send(node) => {
                visit_send_node(&attribute_index, &method_index, &lvar_index, node);
            }
            Node::BuildStruct(node) => {
                visit_build_struct_node(
                    &attribute_index,
                    &method_index,
                    &lvar_index,
                    node,
                    struct_index,
                );
            }
            Node::Const(_) => {}
            _ => todo!(),
        };
    });

    let return_type = struct_index
        .get(&build_struct_node.name)
        .unwrap()
        .return_type
        .clone();
    Some(return_type)
}

pub fn pajama_class_name(base_type: &BaseType) -> String {
    match base_type {
        BaseType::Array(_, _) => "Array".to_string(),
        BaseType::Byte => "Byte".to_string(),
        BaseType::BytePtr => "BytePtr".to_string(),
        BaseType::Class(class_name) => class_name.to_string(),
        BaseType::Int => "Int".to_string(),
        BaseType::Int16 => "Int16".to_string(),
        BaseType::Int32 => "Int32".to_string(),
        BaseType::Int64 => "Int64".to_string(),
        BaseType::Void => "".to_string(),
        BaseType::Struct(_) => "Struct".to_string(),
        BaseType::FnRef => "FnRef".to_string(),
    }
}
