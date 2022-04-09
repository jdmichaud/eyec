const fs = require('fs');

function getNodeAttribute(file) {
  const toAttributes = {};
  if (file.type === 'Executable') {
    toAttributes.shape = 'box';
  }
  if (file.type === 'Library') {
    toAttributes.style = '"filled"';
    toAttributes.fillcolor = '"gray"';
  }
  if (file.type === 'Object') {
    toAttributes.style = '"filled"';
    toAttributes.fillcolor = '"lightgray"';
  }
  return Object.entries(toAttributes).reduce((acc, [key, value]) => acc + `${key}=${value} `, '');
}

function main(argv) {
  const report = JSON.parse(fs.readFileSync(argv[argv.length - 1]));
  console.log('digraph {');
  console.log('rankdir=LR');
  report.stages.forEach(stage => {
    const output = report.files.find(f => f.id === stage.outputs[0]);
    if (output === undefined) {
      return;
    }
    const to = output.name;
    const toAttributes = {};
    if (stage.inputs.length > 1) {
      toAttributes.xlabel = `"${(stage.duration / 1000).toFixed(0)}ms"`;
    }

    stage.inputs.forEach(input => {
      const from = report.files.find(f => f.id === input);
      console.log(`"${from.name}" -> "${to}"`, stage.inputs.length === 1 ? `[ label="${(stage.duration / 1000).toFixed(0)}ms" ];` : '');
      console.log(`"${from.name}" [ ${getNodeAttribute(from)} ];`);
    })
    console.log(`"${to}" [ ${getNodeAttribute(output)} ];`);
  });
  console.log('}');
}

main(process.argv);
