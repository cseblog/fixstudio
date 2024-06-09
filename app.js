document.addEventListener('DOMContentLoaded', () => {
  const fixInput = document.getElementById('fixInput');
  const timelineTable = document.getElementById('timelineTable');
  const detailTable = document.getElementById('detailTable');
  const skipHeartbeats = document.getElementById('skipHeartbeats');
  const skipCommonFields = document.getElementById('skipCommonFields');

  document.getElementById('processBtn').addEventListener('click', () => {
    processFIXData(fixInput.value);
  });

  document.getElementById('clearBtn').addEventListener('click', () => {
    fixInput.value = '';
    timelineTable.innerHTML = '';
    detailTable.innerHTML = '';
  });

  document.getElementById('sampleBtn').addEventListener('click', () => {
    fixInput.value = `8=FIX.4.1\x019=61\x0135=A\x0134=1\x0149=EXEC\x0152=2021105-23:24:06\x0156=BANZAI\x0198=0\x01108=30\x0110=003
8=FIX.4.1\x019=49\x0135=D\x0134=2\x0149=BANZAI\x0152=2021105-23:24:37\x0156=EXEC\x0110=328`;
  });

  function processFIXData(data) {
    const delimiter = detectDelimiter(data);
    const messages = data.split('\n');
    timelineTable.innerHTML = '';
    messages.forEach((message, index) => {
      const fields = parseFIXMessage(message, delimiter);
      const row = document.createElement('tr');
      row.addEventListener('click', () => displayDetail(fields));
      appendCell(row, fields['52']); // Time
      appendCell(row, fields['49']); // Sender
      appendCell(row, fields['56']); // Target
      appendCell(row, fields['35'], getMessageClass(fields['35'])); // Message
      appendCell(row, fields['11']); // Client order ID
      appendCell(row, ''); // Detail button
      timelineTable.appendChild(row);

      if (index === 0) {
        displayDetail(fields);
      }
    });
  }

  function detectDelimiter(data) {
    if (data.includes('\x01')) return '\x01';
    if (data.includes('|')) return '|';
    if (data.includes(' ')) return ' ';
    return '';
  }

  function parseFIXMessage(message, delimiter) {
    const fields = {};
    const pairs = message.split(delimiter);
    pairs.forEach(pair => {
      const [tag, value] = pair.split('=');
      fields[tag] = value;
    });
    return fields;
  }

  function appendCell(row, text, className = '') {
    const cell = document.createElement('td');
    cell.textContent = text || '';
    if (className) cell.classList.add(className);
    row.appendChild(cell);
  }

  function displayDetail(fields) {
    detailTable.innerHTML = '';
    Object.keys(fields).forEach(tag => {
      const row = document.createElement('tr');
      appendCell(row, tag); // Tag
      appendCell(row, getTagDescription(tag), getTagClass(tag)); // Tag Description
      appendCell(row, fields[tag]); // Value
      appendCell(row, ''); // Value Description
      detailTable.appendChild(row);
    });
  }

  function getMessageClass(msgType) {
    const classes = {
      'A': 'logon',
      '0': 'heartbeat',
      'D': 'newordersingle',
      '8': 'erfill',
      '3': 'reject'
    };
    return classes[msgType] || '';
  }

  function getTagClass(tag) {
    const classes = {
      '8': 'beginstring',
      '9': 'bodylength',
      '35': 'msgtype',
      '34': 'msgseqnum',
      '49': 'sendercompid',
      '52': 'sendingtime',
      '56': 'targetcompid',
      '98': 'encryptmethod',
      '108': 'heartbtint',
      '10': 'checksum'
    };
    return classes[tag] || '';
  }

  function getTagDescription(tag) {
    const descriptions = {
      '8': 'BeginString',
      '9': 'BodyLength',
      '35': 'MsgType',
      '34': 'MsgSeqNum',
      '49': 'SenderCompID',
      '52': 'SendingTime',
      '56': 'TargetCompID',
      '98': 'EncryptMethod',
      '108': 'HeartBtInt',
      '10': 'CheckSum'
    };
    return descriptions[tag] || '';
  }
});
