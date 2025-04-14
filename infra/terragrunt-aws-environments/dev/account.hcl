# Set account-wide variables. These are automatically pulled in to configure the remote state bucket in the root
# terragrunt.hcl configuration.
locals {
  account_name   = "qed-it"
  aws_account_id = "" # You can get it from the AWS console
  aws_profile    = "qed-it"
}
