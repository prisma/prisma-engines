use prisma_models::*;

pub struct WriteArgs {
    args: PrismaArgs,
}

impl WriteArgs {
    pub fn new(non_list_args: PrismaArgs) -> WriteArgs {
        WriteArgs { args: non_list_args }
    }

    pub fn non_list_args(&self) -> &PrismaArgs {
        &self.args
    }
}
