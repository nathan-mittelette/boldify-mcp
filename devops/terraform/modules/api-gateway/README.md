<!-- BEGIN_TF_DOCS -->
## Requirements

| Name | Version |
| ---- | ------- |
| <a name="requirement_aws"></a> [aws](#requirement\_aws) | ~> 6.0 |

## Providers

| Name | Version |
| ---- | ------- |
| <a name="provider_aws"></a> [aws](#provider\_aws) | ~> 6.0 |

## Modules

| Name | Source | Version |
| ---- | ------ | ------- |
| <a name="module_dns"></a> [dns](#module\_dns) | ../dns | n/a |

## Resources

| Name | Type |
| ---- | ---- |
| [aws_apigatewayv2_api.apg](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/apigatewayv2_api) | resource |
| [aws_apigatewayv2_stage.default_stage](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/apigatewayv2_stage) | resource |
| [aws_cloudwatch_log_group.api_gw_log_group](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/cloudwatch_log_group) | resource |

## Inputs

| Name | Description | Type | Default | Required |
| ---- | ----------- | ---- | ------- | :------: |
| <a name="input_cors"></a> [cors](#input\_cors) | n/a | <pre>object({<br/>    allow_origins = set(string)<br/>    allow_headers = set(string)<br/>    allow_methods = set(string)<br/>  })</pre> | n/a | yes |
| <a name="input_dns"></a> [dns](#input\_dns) | n/a | <pre>object({<br/>    root_domain = string<br/>    domain_name = string,<br/>  })</pre> | n/a | yes |
| <a name="input_log"></a> [log](#input\_log) | n/a | <pre>object({<br/>    retention = number<br/>  })</pre> | n/a | yes |
| <a name="input_project"></a> [project](#input\_project) | n/a | <pre>object({<br/>    name        = string,<br/>    environment = string,<br/>    provider    = string,<br/>    responsible = string<br/>  })</pre> | n/a | yes |

## Outputs

| Name | Description |
| ---- | ----------- |
| <a name="output_api_gateway"></a> [api\_gateway](#output\_api\_gateway) | The api gateway |
| <a name="output_api_gateway_stage"></a> [api\_gateway\_stage](#output\_api\_gateway\_stage) | The api gateway stage |
| <a name="output_execution_arn"></a> [execution\_arn](#output\_execution\_arn) | The execution arn of the api gateway |
| <a name="output_id"></a> [id](#output\_id) | The id of the api gateway |
<!-- END_TF_DOCS -->