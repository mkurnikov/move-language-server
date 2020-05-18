use anyhow::Result;

use libra_types::access_path::AccessPath;
use libra_types::language_storage::ResourceKey;
use libra_types::write_set::{WriteOp, WriteSet, WriteSetMut};
use move_core_types::account_address::AccountAddress;
use move_core_types::identifier::Identifier;
use move_vm_types::loaded_data::types::{FatStructType, FatType};
use shared::results::{ResourceChange, ResourceChangeOp, ResourceType};

pub struct ResourceStructType(pub FatStructType);

impl Into<ResourceType> for ResourceStructType {
    fn into(self) -> ResourceType {
        let FatStructType {
            address,
            module,
            name,
            ty_args,
            layout,
            ..
        } = self.0;
        ResourceType {
            address: address.to_string(),
            module: module.into_string(),
            name: name.into_string(),
            ty_args: ty_args
                .into_iter()
                .map(|ty_arg| ty_arg.type_tag().unwrap().to_string())
                .collect(),
            layout: layout
                .into_iter()
                .map(|lay_arg| lay_arg.type_tag().unwrap().to_string())
                .collect(),
        }
    }
}

pub fn resource_into_access_path(ty: ResourceType) -> Result<AccessPath> {
    let mut ty_args = Vec::with_capacity(ty.ty_args.len());
    for ty_arg_s in ty.ty_args {
        let quoted = format!(r#""{}""#, ty_arg_s);
        let item = serde_json::from_str::<FatType>(&quoted)
            .unwrap_or_else(|_| panic!("Not a valid ty_arg type {:?}", quoted));
        ty_args.push(item);
    }
    let mut layout = Vec::with_capacity(ty.layout.len());
    for layout_item_s in ty.layout {
        let quoted = format!(r#""{}""#, layout_item_s);
        let item = serde_json::from_str::<FatType>(&quoted)
            .unwrap_or_else(|_| panic!("Not a valid layout type {:?}", quoted));
        layout.push(item);
    }
    let struct_type = FatStructType {
        address: AccountAddress::from_hex_literal(&format!("0x{}", ty.address))?,
        module: Identifier::new(ty.module)?,
        name: Identifier::new(ty.name)?,
        is_resource: true,
        ty_args,
        layout,
    };
    let resource_key = ResourceKey::new(struct_type.address, struct_type.struct_tag().unwrap());
    Ok(AccessPath::resource_access_path(&resource_key))
}

pub struct ResourceWriteOp(pub WriteOp);

impl Into<ResourceChangeOp> for ResourceWriteOp {
    fn into(self) -> ResourceChangeOp {
        match self.0 {
            WriteOp::Value(values) => ResourceChangeOp::SetValue { values },
            WriteOp::Deletion => ResourceChangeOp::Delete,
        }
    }
}

pub fn into_write_op(op: ResourceChangeOp) -> WriteOp {
    match op {
        ResourceChangeOp::SetValue { values } => WriteOp::Value(values),
        ResourceChangeOp::Delete => WriteOp::Deletion,
    }
}

pub fn struct_type_into_access_path(struct_type: FatStructType) -> AccessPath {
    let resource_key = ResourceKey::new(struct_type.address, struct_type.struct_tag().unwrap());
    AccessPath::resource_access_path(&resource_key)
}

pub fn changes_into_writeset(changes: Vec<ResourceChange>) -> Result<WriteSet> {
    let mut write_set = WriteSetMut::default();
    for change in changes {
        let access_path = resource_into_access_path(change.ty)?;
        let op = into_write_op(change.op);
        write_set.push((access_path, op));
    }
    Ok(write_set
        .freeze()
        .expect("WriteSetMut should always be convertible to WriteSet"))
}