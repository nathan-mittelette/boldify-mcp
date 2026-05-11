terraform {

  backend "s3" {
    bucket       = "boldify-mcp-tf-state"
    key          = "terraform/terraform.tfstate"
    region       = "eu-west-1"
    use_lockfile = true
    encrypt      = true
  }

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 6.0"
    }

    local = {
      source  = "hashicorp/local"
      version = "~> 2.1"
    }

    archive = {
      source  = "hashicorp/archive"
      version = "~> 2.0"
    }
  }
}


provider "aws" {
  default_tags {
    tags = module.tags.map
  }
  region = "eu-west-1"
}
