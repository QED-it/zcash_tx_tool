# Background
Those 2 folders contain the Terraform modules used to deploy the infrastructure, and the terragrunt files that uses them.
Each of the folders has it own README.md file with more information.
An AWS profile called 'qed-it' is required to be configured in the AWS CLI for running the terragrunt commands, and the account id should be set in "infra/terragrunt-aws-environments/dev/account.hcl"


After that you'll have to run terragrunt refresh & terraform apply to re-create a new EFS file system drive.
