#!/bin/bash

set -eux

# We check if the .finished file marker exists in the S3 bucket
# i.e. 'all_commits/[COMMIT]/.finished'
object_exists=$(aws s3api head-object --bucket $BUCKET_NAME --key $FILE_PATH || true)

if [ -z "$object_exists" ]; then
echo ".finished file marker was NOT found at $FILE_PATH. Continuing..."
else
echo "::error::.finished file marker was found at $FILE_PATH - This means that artifacts were already uploaded in a previous run. Aborting to avoid overwriting the artifacts.",
exit 1
fi;


# When we were using our Buildkite pipeline
# Before this GitHub Actions pipeline
# We were uploading the artifacts for each build separately
# And the .finished file marker was in the same directory as the build target
# i.e. 'all_commits/[COMMIT]/rhel-openssl-1.1.x/.finished'
object_exists_in_legacy_path=$(aws s3api head-object --bucket $BUCKET_NAME --key $FILE_PATH_LEGACY || true)

if [ -z "$object_exists_in_legacy_path" ]; then
echo "(legacy) .finished file marker was NOT found at $FILE_PATH. Continuing..."
else
echo "::error::(legacy) .finished file marker was found at $FILE_PATH - This means that artifacts were already uploaded in a previous run. Aborting to avoid overwriting the artifacts.",
exit 1
fi;
