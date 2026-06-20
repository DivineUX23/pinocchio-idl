use syn::{ str };

pub struct Idl {
    pub address: str,
    pub metadata: Metadata,
    pub instrcutions: Vec<Instructions>,
    pub accounts: Accounts,
    pub errors: Errors,
    pub types: Types,
    pub constants: Constants
}


pub struct Metadata {

}

pub struct Instructions {
    name: Indent,
    discriminator: u8,
    accounts: Vec<Accounts>,
    args: Option<Vec<Args>>
}

pub struct Accounts {
    pub name: Ident,
    pub writable: bool,
    pub signer: bool,
    
}