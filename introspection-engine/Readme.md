
## Introspect using Binary

Enter db url into introspect.json.

Introspected datamodel  to file
```sh
 cat introspect.json | jq -c | path/to/introspection-engine | jq -r '.result' > datamodel.prisma
 ```

In case of error get full output
```sh
cat introspect.json | jq -c | path/to/introspection-engine
```