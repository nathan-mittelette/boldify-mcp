resource "aws_apigatewayv2_api" "apg" {
  name          = "${local.resources_name}-api"
  protocol_type = "HTTP"

  cors_configuration {
    allow_origins = var.cors.allow_origins
    allow_methods = var.cors.allow_methods
    allow_headers = var.cors.allow_headers
  }
}

resource "aws_apigatewayv2_stage" "default_stage" {
  api_id      = aws_apigatewayv2_api.apg.id
  name        = "$default"
  auto_deploy = true
  access_log_settings {
    destination_arn = aws_cloudwatch_log_group.api_gw_log_group.arn
    format = jsonencode({
      requestId      = "$context.requestId"
      ip             = "$context.identity.sourceIp"
      routeKey       = "$context.routeKey"
      requestTime    = "$context.requestTime"
      httpMethod     = "$context.httpMethod"
      status         = "$context.status"
      protocol       = "$context.protocol"
      responseLength = "$context.responseLength"
    })
  }

  default_route_settings {
    throttling_burst_limit = 50
    throttling_rate_limit  = 50
  }
}

resource "aws_cloudwatch_log_group" "api_gw_log_group" {
  name              = "/aws/api-gateway/${aws_apigatewayv2_api.apg.name}"
  retention_in_days = var.log.retention
}
