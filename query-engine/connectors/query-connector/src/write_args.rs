use prisma_models::*;

pub struct WriteArgs {
    non_list_args: PrismaArgs,
    list_args: Vec<(String, PrismaListValue)>,
}

impl WriteArgs {
    pub fn new(non_list_args: PrismaArgs, list_args: Vec<(String, PrismaListValue)>) -> WriteArgs {
        WriteArgs {
            non_list_args,
            list_args,
        }
    }

    pub fn non_list_args(&self) -> &PrismaArgs {
        &self.non_list_args
    }

    pub fn list_args(&self) -> &Vec<(String, PrismaListValue)> {
        &self.list_args
    }
}