const prismaFmt = require("./js-target/prisma_fmt");
const fs = require("fs")

const file = String(fs.readFileSync("schema.prisma"));

console.log("formatted:")
console.log(prismaFmt.format(file))
console.log("linted:")
console.log(prismaFmt.lint(file))
console.log("native types:")
console.log(prismaFmt.native_types(file))
console.log("preview features:")
console.log(prismaFmt.preview_features(file))
console.log("referential actions")
console.log(prismaFmt.referential_actions(file))
