<!-- BEGIN_TF_DOCS -->
## Requirements

| Name | Version |
| ---- | ------- |
| <a name="requirement_archive"></a> [archive](#requirement\_archive) | ~> 2.0 |
| <a name="requirement_aws"></a> [aws](#requirement\_aws) | ~> 6.0 |
| <a name="requirement_local"></a> [local](#requirement\_local) | ~> 2.1 |

## Providers

No providers.

## Modules

| Name | Source | Version |
| ---- | ------ | ------- |
| <a name="module_api-gateway"></a> [api-gateway](#module\_api-gateway) | ./modules/api-gateway | n/a |
| <a name="module_lambda"></a> [lambda](#module\_lambda) | ./modules/lambda | n/a |
| <a name="module_tags"></a> [tags](#module\_tags) | ./modules/tags | n/a |

## Resources

No resources.

## Inputs

| Name | Description | Type | Default | Required |
| ---- | ----------- | ---- | ------- | :------: |
| <a name="input_api_gateway"></a> [api\_gateway](#input\_api\_gateway) | n/a | <pre>object({<br/>    cors = object({<br/>      allow_origins = set(string)<br/>      allow_headers = set(string)<br/>      allow_methods = set(string)<br/>    }),<br/>    log = object({<br/>      retention = number<br/>    })<br/>  })</pre> | n/a | yes |
| <a name="input_dns"></a> [dns](#input\_dns) | n/a | <pre>object({<br/>    root_domain = string<br/>    domain_name = string<br/>  })</pre> | n/a | yes |
| <a name="input_lambdas"></a> [lambdas](#input\_lambdas) | n/a | <pre>set(object({<br/>    name        = string<br/>    output      = string<br/>    source      = string<br/>    description = string<br/>    memory      = string<br/>    timeout     = string<br/>    runtime     = string<br/>    handler     = string<br/>    http = object({<br/>      method = string<br/>      path   = string<br/>    })<br/>  }))</pre> | n/a | yes |
| <a name="input_project"></a> [project](#input\_project) | n/a | <pre>object({<br/>    name        = string<br/>    environment = string<br/>    provider    = string<br/>    responsible = string<br/>  })</pre> | n/a | yes |

## Outputs

No outputs.
<!-- END_TF_DOCS -->