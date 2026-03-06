--
-- PostgreSQL database dump
--

\restrict CGm1piffUmg16OMfllaGfL8NvX469jf8Mks1gf4u3lGnTGah8ntJlhc3CG8KzL2

-- Dumped from database version 17.8 (Debian 17.8-1.pgdg13+1)
-- Dumped by pg_dump version 17.8 (Debian 17.8-1.pgdg13+1)

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET transaction_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: document_training_lineage; Type: TABLE; Schema: public; Owner: kb_admin
--

CREATE TABLE public.document_training_lineage (
    document_id uuid NOT NULL,
    cycle_id uuid NOT NULL
);


ALTER TABLE public.document_training_lineage OWNER TO kb_admin;

--
-- Name: documents; Type: TABLE; Schema: public; Owner: kb_admin
--

CREATE TABLE public.documents (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    source text NOT NULL,
    domain text NOT NULL,
    doc_type text DEFAULT 'documentation'::text NOT NULL,
    content text NOT NULL,
    content_length integer NOT NULL,
    content_hash text NOT NULL,
    inserted_by text DEFAULT 'agent'::text NOT NULL,
    collected_at timestamp with time zone DEFAULT now() NOT NULL,
    training_eligible boolean DEFAULT false,
    used_in_training boolean DEFAULT false
);


ALTER TABLE public.documents OWNER TO kb_admin;

--
-- Name: models; Type: TABLE; Schema: public; Owner: kb_admin
--

CREATE TABLE public.models (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    name text NOT NULL,
    domain text NOT NULL,
    base_model text NOT NULL,
    status text DEFAULT 'training'::text NOT NULL,
    dataset_size integer NOT NULL,
    training_steps integer,
    benchmark_score real,
    adapter_checksum text,
    adapter_path text,
    training_cycle_id uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    approved_at timestamp with time zone,
    deployed_at timestamp with time zone
);


ALTER TABLE public.models OWNER TO kb_admin;

--
-- Name: training_cycles; Type: TABLE; Schema: public; Owner: kb_admin
--

CREATE TABLE public.training_cycles (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    domain text NOT NULL,
    base_model text NOT NULL,
    status text DEFAULT 'running'::text NOT NULL,
    config jsonb NOT NULL,
    dataset_size integer,
    final_loss real,
    started_at timestamp with time zone DEFAULT now() NOT NULL,
    completed_at timestamp with time zone
);


ALTER TABLE public.training_cycles OWNER TO kb_admin;

--
-- Name: validation; Type: TABLE; Schema: public; Owner: kb_admin
--

CREATE TABLE public.validation (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    document_id uuid NOT NULL,
    status text DEFAULT 'pending'::text NOT NULL,
    decided_by text DEFAULT 'pending'::text NOT NULL,
    decided_at timestamp with time zone,
    rejection_reason text,
    tags text[] DEFAULT '{}'::text[]
);


ALTER TABLE public.validation OWNER TO kb_admin;

--
-- Name: document_training_lineage document_training_lineage_pkey; Type: CONSTRAINT; Schema: public; Owner: kb_admin
--

ALTER TABLE ONLY public.document_training_lineage
    ADD CONSTRAINT document_training_lineage_pkey PRIMARY KEY (document_id, cycle_id);


--
-- Name: documents documents_pkey; Type: CONSTRAINT; Schema: public; Owner: kb_admin
--

ALTER TABLE ONLY public.documents
    ADD CONSTRAINT documents_pkey PRIMARY KEY (id);


--
-- Name: models models_pkey; Type: CONSTRAINT; Schema: public; Owner: kb_admin
--

ALTER TABLE ONLY public.models
    ADD CONSTRAINT models_pkey PRIMARY KEY (id);


--
-- Name: training_cycles training_cycles_pkey; Type: CONSTRAINT; Schema: public; Owner: kb_admin
--

ALTER TABLE ONLY public.training_cycles
    ADD CONSTRAINT training_cycles_pkey PRIMARY KEY (id);


--
-- Name: validation validation_pkey; Type: CONSTRAINT; Schema: public; Owner: kb_admin
--

ALTER TABLE ONLY public.validation
    ADD CONSTRAINT validation_pkey PRIMARY KEY (id);


--
-- Name: idx_documents_domain; Type: INDEX; Schema: public; Owner: kb_admin
--

CREATE INDEX idx_documents_domain ON public.documents USING btree (domain);


--
-- Name: idx_documents_source; Type: INDEX; Schema: public; Owner: kb_admin
--

CREATE INDEX idx_documents_source ON public.documents USING btree (source);


--
-- Name: idx_documents_training_eligible; Type: INDEX; Schema: public; Owner: kb_admin
--

CREATE INDEX idx_documents_training_eligible ON public.documents USING btree (training_eligible);


--
-- Name: idx_models_domain; Type: INDEX; Schema: public; Owner: kb_admin
--

CREATE INDEX idx_models_domain ON public.models USING btree (domain);


--
-- Name: idx_models_status; Type: INDEX; Schema: public; Owner: kb_admin
--

CREATE INDEX idx_models_status ON public.models USING btree (status);


--
-- Name: idx_training_cycles_domain; Type: INDEX; Schema: public; Owner: kb_admin
--

CREATE INDEX idx_training_cycles_domain ON public.training_cycles USING btree (domain);


--
-- Name: idx_validation_document_id; Type: INDEX; Schema: public; Owner: kb_admin
--

CREATE INDEX idx_validation_document_id ON public.validation USING btree (document_id);


--
-- Name: idx_validation_status; Type: INDEX; Schema: public; Owner: kb_admin
--

CREATE INDEX idx_validation_status ON public.validation USING btree (status);


--
-- Name: idx_validation_tags; Type: INDEX; Schema: public; Owner: kb_admin
--

CREATE INDEX idx_validation_tags ON public.validation USING gin (tags);


--
-- Name: document_training_lineage document_training_lineage_cycle_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: kb_admin
--

ALTER TABLE ONLY public.document_training_lineage
    ADD CONSTRAINT document_training_lineage_cycle_id_fkey FOREIGN KEY (cycle_id) REFERENCES public.training_cycles(id);


--
-- Name: document_training_lineage document_training_lineage_document_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: kb_admin
--

ALTER TABLE ONLY public.document_training_lineage
    ADD CONSTRAINT document_training_lineage_document_id_fkey FOREIGN KEY (document_id) REFERENCES public.documents(id);


--
-- Name: models models_training_cycle_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: kb_admin
--

ALTER TABLE ONLY public.models
    ADD CONSTRAINT models_training_cycle_id_fkey FOREIGN KEY (training_cycle_id) REFERENCES public.training_cycles(id);


--
-- Name: validation validation_document_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: kb_admin
--

ALTER TABLE ONLY public.validation
    ADD CONSTRAINT validation_document_id_fkey FOREIGN KEY (document_id) REFERENCES public.documents(id);


--
-- Name: SCHEMA public; Type: ACL; Schema: -; Owner: pg_database_owner
--

GRANT USAGE ON SCHEMA public TO kb_ingest;
GRANT USAGE ON SCHEMA public TO kb_reader;


--
-- Name: TABLE document_training_lineage; Type: ACL; Schema: public; Owner: kb_admin
--

GRANT SELECT,INSERT,UPDATE ON TABLE public.document_training_lineage TO kb_ingest;
GRANT SELECT ON TABLE public.document_training_lineage TO kb_reader;


--
-- Name: TABLE documents; Type: ACL; Schema: public; Owner: kb_admin
--

GRANT SELECT,INSERT,UPDATE ON TABLE public.documents TO kb_ingest;
GRANT SELECT ON TABLE public.documents TO kb_reader;


--
-- Name: TABLE models; Type: ACL; Schema: public; Owner: kb_admin
--

GRANT SELECT,INSERT,UPDATE ON TABLE public.models TO kb_ingest;
GRANT SELECT ON TABLE public.models TO kb_reader;


--
-- Name: TABLE training_cycles; Type: ACL; Schema: public; Owner: kb_admin
--

GRANT SELECT,INSERT,UPDATE ON TABLE public.training_cycles TO kb_ingest;
GRANT SELECT ON TABLE public.training_cycles TO kb_reader;


--
-- Name: TABLE validation; Type: ACL; Schema: public; Owner: kb_admin
--

GRANT SELECT,INSERT,UPDATE ON TABLE public.validation TO kb_ingest;
GRANT SELECT ON TABLE public.validation TO kb_reader;


--
-- PostgreSQL database dump complete
--

\unrestrict CGm1piffUmg16OMfllaGfL8NvX469jf8Mks1gf4u3lGnTGah8ntJlhc3CG8KzL2

