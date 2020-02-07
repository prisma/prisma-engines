#!/bin/bash
#echo "{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"introspect\",\"params\":[{\"url\":\"file:../db/lift.db\"}]}" | ../target/debug/introspection-engine
echo "{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"getDatabaseDescription\",\"params\":[{\"url\":\"file:../db/lift.db\"}]}" | ../target/debug/introspection-engine
# debug
# echo "{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"introspect\",\"params\":[{\"url\":\"file:${folder}${fileName}.db\"}]}" | ../target/debug/introspection-engine 

# write structopt cli that can test all the rpc calls
# list databases
# introspect
# getDatabaseDescription
# getMetadata