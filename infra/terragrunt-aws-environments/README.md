# Infrastructure Deployment Makefile

This repository includes a `Makefile` to streamline the deployment, management, and teardown of AWS infrastructure using Terragrunt. The script ensures all prerequisites are met and simplifies executing commands for planning, applying, and destroying infrastructure across all modules in a specific environment.
After creating the infrastructure, which includes the ECR repository, you can use the push-deploy Github workflow to deploy the Zebra Server to ECR and the ECS cluster.
You can see the workflow in this repository's `.github/workflows/push-deploy.yaml` file.

## Prerequisites

Before using this script, ensure the following:

1. **AWS CLI**:
   - Install the AWS CLI.
   - Configure it with your credentials.
   - Ensure the `qed-it` AWS profile exists in `~/.aws/credentials`.

2. **Terragrunt**:
   - Install Terragrunt: [Install Instructions](https://terragrunt.gruntwork.io/docs/getting-started/install/).

3. **Make**:
   - Ensure `make` is installed on your system.

4. **Repository Structure**:
   - The script expects the `infra/terragrunt-aws-environments` directory to exist at the following location:
     ```
     ./zebra/demo-deploy/infra/terragrunt-aws-environments
     ```
   - Update the `Makefile` if the directory structure changes.

## Makefile Targets

### 1. `check-prerequisites`
- Verifies that the required tools and configurations are available:
  - AWS CLI is installed.
  - Terragrunt is installed.
  - The `qed-it` AWS profile exists.

### 2. `plan-all`
- **Command**: `make plan-all`
- Plans changes for all modules in the environment specified in the `Makefile`.

### 3. `apply-all`
- **Command**: `make apply-all`
- Applies the planned changes for all modules in the environment.

### 4. `destroy-all`
- **Command**: `make destroy-all`
- Destroys all resources in the specified environment.

## Usage

1. Navigate to the directory containing the `Makefile`:
   ```bash
   ./zebra/demo-deploy

## Public ECR Repository Info

To view the public ECR image metadata and download options, visit the repository's gallery page:

👉 [Zcash Transaction Tool on ECR Gallery](https://gallery.ecr.aws/j7v0v6n9/tx-tool)

> This page includes the description, usage instructions, supported architectures, and OS information.

### Updating ECR Public Metadata

Terraform currently **does not support** updating the catalog metadata for public ECR repositories. To modify the metadata (e.g., description, usage text, etc.), use the AWS CLI:

```bash
aws ecr-public put-repository-catalog-data \
  --region us-east-1 \
  --repository-name tx-tool \
  --catalog-data '{
    "description": "Zcash transaction tool",
    "aboutText": "CLI docker tool for interacting with the Zebra node. Made by qedit.",
    "usageText": "Run as a docker container, with the regular parameters of the tx-tool to choose which node to connect to (ZCASH_NODE_ADDRESS, ZCASH_NODE_PORT, ZCASH_NODE_PROTOCOL)",
    "architectures": ["x86_64"],
    "operatingSystems": ["Linux"]
  }'
