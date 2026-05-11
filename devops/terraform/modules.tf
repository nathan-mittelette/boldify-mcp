module "api-gateway" {
  source = "./modules/api-gateway"

  project = var.project

  cors = var.api_gateway.cors
  log  = var.api_gateway.log

  dns = var.dns
}

module "tags" {
  source = "./modules/tags"

  project = var.project
}

module "lambda" {
  for_each = { for lambda in var.lambdas : lambda.name => lambda }
  source   = "./modules/lambda"

  project = var.project
  lambda  = each.value

  api_gateway = {
    execution_arn = module.api-gateway.execution_arn
    id            = module.api-gateway.id
  }
}
