FROM tiangolo/uvicorn-gunicorn:python3.10 AS bitvmx-base

RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    git \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install Python dependencies
RUN pip install --upgrade pip

COPY requirements/base.txt /tmp/requirements/
RUN pip install -r /tmp/requirements/base.txt

# Install python-bitcoin-utils
RUN pip install --no-cache-dir git+https://github.com/ramonamela/python-bitcoin-utils.git@3e8c2a0bf1b080ed5f80f4e78f71005f42eb52c5

# Install maturin for building Python extensions
RUN pip install maturin

# Use mock pybitvmbinding module to avoid dependency conflicts
COPY pybitvmbinding /app/pybitvmbinding
ENV PYTHONPATH="/app:${PYTHONPATH}"

# Setup working directory
RUN mkdir /bitvmx-backend
WORKDIR /bitvmx-backend

# Copy project files
COPY ./BitVMX-CPU ./BitVMX-CPU
COPY ./blockchain_query_services ./blockchain_query_services
COPY ./bitvmx_protocol_library ./bitvmx_protocol_library

# Build BitVMX-CPU
WORKDIR /bitvmx-backend/BitVMX-CPU
RUN cargo build --release

WORKDIR /bitvmx-backend

# Prover stage
FROM bitvmx-base as prover-backend

COPY requirements/prover.txt /tmp/requirements/
RUN pip install -r /tmp/requirements/prover.txt

COPY ./prover_app ./prover_app
RUN mkdir -p prover_files

CMD ["uvicorn", "prover_app.main:app", "--host", "0.0.0.0", "--port", "80"]

# Verifier stage
FROM bitvmx-base as verifier-backend

COPY requirements/verifier.txt /tmp/requirements/
RUN pip install -r /tmp/requirements/verifier.txt

COPY ./verifier_app ./verifier_app
RUN mkdir -p verifier_files

CMD ["uvicorn", "verifier_app.main:app", "--host", "0.0.0.0", "--port", "80"]