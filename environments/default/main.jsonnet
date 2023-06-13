local k = import 'github.com/grafana/jsonnet-libs/ksonnet-util/kausal.libsonnet';
local deployment = k.apps.v1.deployment;
local container = k.core.v1.container;
local port = k.core.v1.containerPort;

function(secrets_yaml) {
  local portName = 'http-metrics',
  local cfg = $._config,

  _config:: {
    local config = self,
    secrets: std.native('parseYaml')(secrets_yaml)[0],
    name: 'vatsim-exporter',
    image: 'registry.git.mayflower.de/franz.pletz/vatsim-exporter/vatsim-prometheus-exporter:%s' % std.extVar('commit_hash'),
  },

  deployment: deployment.new(
    name=cfg.name,
    replicas=1,
    containers=[
      container.new(cfg.name, cfg.image)
      + container.withPorts([port.new(portName, 9185)]),
    ],
  ) + deployment.spec.template.spec.withImagePullSecrets({ name: 'image-pull-secret' }),
  service: k.util.serviceFor(self.deployment, nameFormat='%(port)s'),
  imagePullSecret: k.core.v1.secret.new(
    'image-pull-secret',
    null,
    'kubernetes.io/dockerconfigjson'
  ) + k.core.v1.secret.withStringData({
    '.dockerconfigjson': $._config.secrets.image_pull_secret,
  }),

  serviceMonitor: {
    apiVersion: 'monitoring.coreos.com/v1',
    kind: 'ServiceMonitor',
    metadata: {
      name: '%s-service-monitor' % cfg.name,
    },
    spec: {
      endpoints: [
        {
          port: portName,
          path: '/metrics',
        },
      ],
      selector: {
        matchExpressions: [{
          key: 'name',
          operator: 'In',
          values: [cfg.name],
        }],
      },
    },
  },

}
