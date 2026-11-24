# Test certificates for the PostgreSQL `sslcert` tests

These files are fixtures for the unit tests in `../mod.rs`. They are not used
outside the tests.

- `ca-a.pem`, `ca-b.pem`: two self-signed test CAs. Their private keys were
  discarded and are not in the repository.
- `server.pem`: a certificate for `localhost` (`subjectAltName=DNS:localhost`
  only) signed by CA B.
- `server-key.pem`: the private key of `server.pem`. **Test-only**: it is used
  by the in-process TLS acceptor in the tests and protects nothing.
- `bundle-b-second.pem`: CA A, a comment line, then CA B.
- `bundle-b-first.pem`: CA B, then CA A.
- `bundle-corrupt-tail.pem`: CA A, then a copy of CA B with corrupted base64.
- `not-a-cert.pem`: plain text.

The CA certificates are valid for 100 years. `server.pem` is valid from
2019-01-01 to 2118-12-31. macOS limits TLS server certificates whose `notBefore`
is on or after 2019-07-01 to 825 days, and the earlier `notBefore` keeps this
long-lived fixture accepted there too. They were generated with OpenSSL 3.5:

```sh
openssl req -x509 -newkey rsa:2048 -nodes -keyout ca-a.key -out ca-a.pem -days 36500 \
  -subj "/CN=Test CA A" -addext "basicConstraints=critical,CA:TRUE" \
  -addext "keyUsage=critical,keyCertSign,cRLSign"
openssl req -x509 -newkey rsa:2048 -nodes -keyout ca-b.key -out ca-b.pem -days 36500 \
  -subj "/CN=Test CA B" -addext "basicConstraints=critical,CA:TRUE" \
  -addext "keyUsage=critical,keyCertSign,cRLSign"
openssl req -newkey rsa:2048 -nodes -keyout server-key.pem -out server.csr -subj "/CN=localhost"
printf 'subjectAltName=DNS:localhost\nbasicConstraints=CA:FALSE\nkeyUsage=digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n' > ext.cnf
openssl x509 -req -in server.csr -CA ca-b.pem -CAkey ca-b.key -CAcreateserial \
  -out server.pem -not_before 20190101000000Z -not_after 21181231000000Z -extfile ext.cnf
{ cat ca-a.pem; echo "# Test CA B follows"; cat ca-b.pem; } > bundle-b-second.pem
cat ca-b.pem ca-a.pem > bundle-b-first.pem
{ cat ca-a.pem; sed '5s/.\{8\}/!!!!!!!!/' ca-b.pem; } > bundle-corrupt-tail.pem
echo "this is not a certificate" > not-a-cert.pem
rm ca-a.key ca-b.key ca-b.srl server.csr ext.cnf
```
