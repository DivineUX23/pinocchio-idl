use serde::Serialize;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Idl {
    pub address: String,
    pub metadata: Metadata,
    pub instructions: Vec<IdlInstruction>,
    pub accounts: Vec<IdlAccountDef>,
    pub errors: Vec<IdlError>,
    pub types: Vec<IdlTypeDefinition>,
    pub constants: Vec<IdlConstant>
}



#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub name: String,
    pub version: String,
    pub spec: String,
    pub description: String,
}



#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IdlInstruction {
    pub name: String,
    pub discriminator: Vec<u8>,
    pub accounts: Vec<IdlAccount>,
    pub args: Option<Vec<IdlArg>>
}



#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IdlAccount {
    pub name: String,
    
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub writable: bool,

    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub signer: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub relations: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pda_seeds: Option<IdlPda>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}


#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IdlArg {
    pub name: String,
    pub r#type: String
}



#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IdlPda {
    pub seeds: Vec<IdlPdaSeed>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub program: Option<IdlPdaProgram>
}



#[derive(Serialize, Debug)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum IdlPdaSeed {
    Account {
        path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        account: Option<String>,
    },
    Arg {
        path: String,
    },
    Const {
        value: Vec<u8>,
    },
}



#[derive(Serialize, Debug)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum IdlPdaProgram {
    Const { value: Vec<u8> }
}



#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IdlAccountDef {
    pub name: String,
    pub discriminator: Vec<u8>
}



#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IdlError {
    pub code: u32,
    pub name: String,
    pub msg: String
}



#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IdlTypeDefinition {
    pub name: String,
    pub r#type: IdlType
}



#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IdlType {
    pub kind: String,
    pub fields: Vec<IdlField>
}



#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IdlField {
    pub name: String,
    pub r#type: String,
}



#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IdlConstant {
    pub name: String,
    pub r#type: String,
    pub value: String
}