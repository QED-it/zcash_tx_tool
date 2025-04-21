locals {
  # Automatically load environment-level variables
  environment_vars = read_terragrunt_config(find_in_parent_folders("env.hcl"))

  region_vars = read_terragrunt_config(find_in_parent_folders("region.hcl"))
  account_vars = read_terragrunt_config(find_in_parent_folders("account.hcl"))
  # Extract out common variables for reuse
  env = local.environment_vars.locals.environment
}

# Terragrunt will copy the Terraform configurations specified by the source parameter, along with any files in the
# working directory, into a temporary folder, and execute your Terraform commands in that folder.
terraform {
  source = "../../../../../terraform-aws-modules/ecr-public"
}

# Include all settings from the root terragrunt.hcl file
include {
  path = find_in_parent_folders()
}

inputs = {
  env = local.env
  name = "tx-tool"
  description = "Qedit tx-tool is a tool for testing the Zebra node and showcasing its capabilities"
  usage_text = "Run the docker image with ZCASH_NODE_ADDRESS, ZCASH_NODE_PORT, ZCASH_NODE_PROTOCOL arguments to connect to the Zebra node"
  about_text = "Qedit tx-tool"
  architecture = "ARM"
  operating_system = "Linux"
  aws_account_id = local.account_vars.locals.aws_account_id
}
