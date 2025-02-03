# Project Overview

This repository contains workflows designed to manage Docker images with Amazon Elastic Container Registry (ECR) and execute a tool using these Docker images. Below is an overview of the two main workflows:

## `push-to-ecr.yaml`

This workflow is configured to build, tag, and push Docker images to Amazon ECR. It automates the process of taking code changes through the following key steps:

- **Checkout**: Checks out the latest code from the repository.
- **Configure AWS Credentials**: Sets up the necessary AWS credentials for interacting with AWS services.
- **Login to Amazon ECR**: Logs into the Amazon ECR service.
- **Build, Tag, and Push Docker Image**: Builds a Docker image using the Dockerfile, tags it with the latest and current commit identifiers, and pushes these tags to ECR.

This workflow is manually triggered to provide flexibility and control over when Docker images are pushed to ECR.

## `run-tx-tool.yaml`

This workflow is designed to run a Docker container from an image stored in Amazon ECR with specific inputs for a Zcash node. The key steps include:

- **Configure AWS Credentials**: Like the previous workflow, this also configures AWS credentials.
- **Login and Pull Docker Image**: Logs into ECR and pulls the specified Docker image.
- **Run Docker Container**: Runs a Docker container, passing environment variables related to the Zcash node.

Similarly, this workflow is manually triggered, allowing users to specify when and with which parameters the container runs.

These workflows leverage GitHub Actions for continuous integration and deployment processes, ensuring consistent operations across various environments.

# Repository Variables

Default repository variables used in these workflows:

* `AWS_REGION`: AWS region for ECR and other services. Default: `eu-central-1`
* `ECR_REGISTRY_ALIAS`: ECR registry alias/ID. Default: `j7v0v6n9` 
* `ECR_REPOSITORY`: ECR repository name. Default: `tx-tool`

These variables ensure consistency and maintainability by reducing hardcoded values.
