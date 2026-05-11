project = {
  name        = "boldify-mcp",
  environment = "prod",
  responsible = "mittelette.nathan@gmail.com"
  provider    = "opentofu"
}

api_gateway = {
  cors = {
    allow_headers = ["*"],
    allow_origins = ["*"],
    allow_methods = ["OPTIONS", "GET", "POST", "DELETE", "PUT"]
  },
  log = {
    retention = 30
  }
}

dns = {
  root_domain = "boldify.net"
  domain_name = "api.boldify.net"
}

lambdas = [{
  name        = "api-convert"
  description = "Lambda to convert HTML or Markdown to ASCII formatted text"
  handler     = "index.handler",
  memory      = "128",
  timeout     = "30",
  runtime     = "provided.al2023",
  source      = "build/api-convert"
  output      = "build"
  https = [{
    method = "POST"
    path   = "/api/convert"
  }]
  },
  {
    name        = "api-syntaxes"
    description = "Lambda to get HTML or Markdown syntaxes"
    handler     = "index.handler",
    memory      = "128",
    timeout     = "30",
    runtime     = "provided.al2023",
    source      = "build/api-syntaxes"
    output      = "build"
    https = [{
      method = "GET"
      path   = "/api/syntaxes"
    }]
  },
  {
    name        = "mcp-http"
    description = "MCP"
    handler     = "bootstrap",
    memory      = "128",
    timeout     = "30",
    runtime     = "provided.al2023",
    source      = "build/mcp-http"
    output      = "build"
    https = [{
      method = "ANY"
      path   = "/mcp"
      }, {
      method = "ANY",
      path   = "/mcp/{proxy+}"
      }, {
      method = "GET",
      path   = "/health"
    }]
    layers = ["arn:aws:lambda:eu-west-1:753240598075:layer:LambdaAdapterLayerX86:23"]
    environments = {
      AWS_LAMBDA_EXEC_WRAPPER : "/opt/bootstrap"
      AWS_LWA_ENABLE_COMPRESSION : "true"
      AWS_LWA_PORT : "8080"
      AWS_LWA_READINESS_CHECK_PATH : "/health"
      AWS_LWA_INVOKE_MODE : "buffered"
    }
}]
