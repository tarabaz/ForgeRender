// Shim minimo tra ForgeRender (Rust) e Open CASCADE Technology.
//
// Legge un file STEP con XCAF (gerarchia dell'assieme, nomi, trasformazioni,
// colori), triangola ogni pezzo una sola volta e passa i dati a Rust tramite
// callback C. Nessuna eccezione C++ attraversa il confine: tutte vengono
// catturate e trasformate in un messaggio d'errore.

#include <cstdio>
#include <BRepLib_ToolTriangulatedShape.hxx>
#include <BRepMesh_IncrementalMesh.hxx>
#include <BRep_Tool.hxx>
#include <Poly_Triangulation.hxx>
#include <Quantity_Color.hxx>
#include <STEPCAFControl_Reader.hxx>
#include <Standard_Failure.hxx>
#include <TDF_LabelSequence.hxx>
#include <TDF_Tool.hxx>
#include <TDataStd_Name.hxx>
#include <TDocStd_Document.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Face.hxx>
#include <XCAFApp_Application.hxx>
#include <XCAFDoc_ColorTool.hxx>
#include <XCAFDoc_DocumentTool.hxx>
#include <XCAFDoc_ShapeTool.hxx>

#include <cstring>
#include <map>
#include <string>
#include <vector>

extern "C" {

typedef struct FrStepCallbacks {
    void* user;
    // Restituisce l'indice della mesh creata.
    int (*mesh)(void* user, const float* positions, const float* normals, int vertex_count, const unsigned* indices,
                int triangle_count);
    // Restituisce l'indice del nodo creato. `matrix` è 4x4 column-major (traslazione in 12,13,14).
    int (*node)(void* user, int parent, const char* name, const double* matrix, int has_color, float r, float g, float b,
                int mesh);
    void (*progress)(void* user, const char* message);
} FrStepCallbacks;

int fr_step_convert(const char* path_utf8, double linear_deflection, double angular_deflection,
                    const FrStepCallbacks* cb, double* unit_in_meters, char* error, int error_len);
}

namespace {

struct Ctx {
    const FrStepCallbacks* cb;
    Handle(XCAFDoc_ShapeTool) shapes;
    Handle(XCAFDoc_ColorTool) colors;
    double lin;
    double ang;
    // Prototipo (entry del label) -> mesh già triangolate, una per gruppo di colore.
    std::map<std::string, std::vector<std::pair<int, int>>> meshes;  // (mesh, indice colore)
    std::vector<Quantity_Color> palette;
    int parts = 0;
};

std::string label_name(const TDF_Label& l) {
    Handle(TDataStd_Name) n;
    if (!l.FindAttribute(TDataStd_Name::GetID(), n)) return std::string();
    const TCollection_ExtendedString& e = n->Get();
    std::string out(e.LengthOfCString() + 1, '\0');
    Standard_PCharacter p = &out[0];
    e.ToUTF8CString(p);
    out.resize(std::strlen(out.c_str()));
    return out;
}

std::string label_entry(const TDF_Label& l) {
    TCollection_AsciiString s;
    TDF_Tool::Entry(l, s);
    return std::string(s.ToCString());
}

bool label_color(Ctx& c, const TDF_Label& l, Quantity_Color& col) {
    return c.colors->GetColor(l, XCAFDoc_ColorSurf, col) || c.colors->GetColor(l, XCAFDoc_ColorGen, col);
}

bool shape_color(Ctx& c, const TopoDS_Shape& s, Quantity_Color& col) {
    return c.colors->GetColor(s, XCAFDoc_ColorSurf, col) || c.colors->GetColor(s, XCAFDoc_ColorGen, col);
}

int palette_index(Ctx& c, const Quantity_Color& col) {
    for (size_t i = 0; i < c.palette.size(); ++i)
        if (c.palette[i].IsEqual(col)) return (int)i;
    c.palette.push_back(col);
    return (int)c.palette.size() - 1;
}

void to_matrix(const TopLoc_Location& loc, double m[16]) {
    const gp_Trsf t = loc.Transformation();
    for (int col = 0; col < 4; ++col)
        for (int row = 0; row < 4; ++row) m[col * 4 + row] = 0.0;
    for (int row = 1; row <= 3; ++row) {
        for (int col = 1; col <= 3; ++col) m[(col - 1) * 4 + (row - 1)] = t.Value(row, col);
        m[12 + (row - 1)] = t.Value(row, 4);
    }
    m[15] = 1.0;
}

// Triangola un prototipo e ne restituisce le mesh, raggruppando le facce per colore.
const std::vector<std::pair<int, int>>& part_meshes(Ctx& c, const TDF_Label& proto) {
    const std::string key = label_entry(proto);
    auto it = c.meshes.find(key);
    if (it != c.meshes.end()) return it->second;

    std::vector<std::pair<int, int>> result;
    TopoDS_Shape shape = XCAFDoc_ShapeTool::GetShape(proto);
    if (!shape.IsNull()) {
        BRepMesh_IncrementalMesh mesher(shape, c.lin, Standard_True, c.ang, Standard_True);
        Quantity_Color part_col;
        const bool has_part_col = label_color(c, proto, part_col) || shape_color(c, shape, part_col);
        const int part_idx = has_part_col ? palette_index(c, part_col) : -1;

        struct Group {
            std::vector<float> pos, nrm;
            std::vector<unsigned> idx;
        };
        std::map<int, Group> groups;
        for (TopExp_Explorer ex(shape, TopAbs_FACE); ex.More(); ex.Next()) {
            const TopoDS_Face face = TopoDS::Face(ex.Current());
            TopLoc_Location floc;
            Handle(Poly_Triangulation) tri = BRep_Tool::Triangulation(face, floc);
            if (tri.IsNull() || tri->NbTriangles() == 0) continue;
            if (!tri->HasNormals()) BRepLib_ToolTriangulatedShape::ComputeNormals(face, tri);
            Quantity_Color fcol;
            int g = shape_color(c, face, fcol) ? palette_index(c, fcol) : part_idx;
            Group& grp = groups[g];
            const gp_Trsf tr = floc.Transformation();
            const bool reversed = face.Orientation() == TopAbs_REVERSED;
            const unsigned base = (unsigned)(grp.pos.size() / 3);
            for (int i = 1; i <= tri->NbNodes(); ++i) {
                gp_Pnt p = tri->Node(i).Transformed(tr);
                gp_Dir n = tri->HasNormals() ? tri->Normal(i) : gp_Dir(0, 0, 1);
                n.Transform(tr);
                if (reversed) n.Reverse();
                grp.pos.insert(grp.pos.end(), {(float)p.X(), (float)p.Y(), (float)p.Z()});
                grp.nrm.insert(grp.nrm.end(), {(float)n.X(), (float)n.Y(), (float)n.Z()});
            }
            for (int t = 1; t <= tri->NbTriangles(); ++t) {
                int a, b, d;
                tri->Triangle(t).Get(a, b, d);
                if (reversed) std::swap(b, d);
                grp.idx.insert(grp.idx.end(), {base + a - 1, base + b - 1, base + d - 1});
            }
        }
        for (auto& [color, grp] : groups) {
            if (grp.idx.empty()) continue;
            int m = c.cb->mesh(c.cb->user, grp.pos.data(), grp.nrm.data(), (int)(grp.pos.size() / 3), grp.idx.data(),
                               (int)(grp.idx.size() / 3));
            result.emplace_back(m, color);
        }
        if (++c.parts % 25 == 0 && c.cb->progress) {
            std::string msg = "pezzi triangolati: " + std::to_string(c.parts);
            c.cb->progress(c.cb->user, msg.c_str());
        }
    }
    return c.meshes.emplace(key, std::move(result)).first->second;
}

int emit_node(Ctx& c, int parent, const std::string& name, const TopLoc_Location& loc, const Quantity_Color* col,
              int mesh) {
    double m[16];
    to_matrix(loc, m);
    float r = 0, g = 0, b = 0;
    if (col) {
        Standard_Real rr, gg, bb;
        col->Values(rr, gg, bb, Quantity_TOC_RGB);  // RGB lineare
        r = (float)rr;
        g = (float)gg;
        b = (float)bb;
    }
    return c.cb->node(c.cb->user, parent, name.c_str(), m, col ? 1 : 0, r, g, b, mesh);
}

// Visita un prototipo (pezzo o sotto-assieme) già rappresentato dal nodo `node`.
void visit_proto(Ctx& c, const TDF_Label& proto, int node, const std::string& name) {
    if (XCAFDoc_ShapeTool::IsAssembly(proto)) {
        TDF_LabelSequence comps;
        XCAFDoc_ShapeTool::GetComponents(proto, comps);
        for (int i = 1; i <= comps.Length(); ++i) {
            const TDF_Label comp = comps.Value(i);
            TDF_Label ref;
            if (!XCAFDoc_ShapeTool::GetReferredShape(comp, ref)) continue;
            std::string inst = label_name(comp);
            if (inst.empty()) inst = label_name(ref);
            if (inst.empty()) inst = "componente-" + std::to_string(i);
            Quantity_Color col;
            const bool has_col = label_color(c, comp, col);
            const int child = emit_node(c, node, inst, XCAFDoc_ShapeTool::GetLocation(comp), has_col ? &col : nullptr, -1);
            visit_proto(c, ref, child, inst);
        }
        return;
    }
    const auto& ms = part_meshes(c, proto);
    TopLoc_Location identity;
    int k = 0;
    for (const auto& [mesh, color] : ms) {
        const Quantity_Color* col = color >= 0 ? &c.palette[color] : nullptr;
        // Una mesh: figlio "corpo"; più colori: un figlio per colore.
        std::string sub = ms.size() == 1 ? name + " (corpo)" : name + " · colore " + std::to_string(++k);
        emit_node(c, node, sub, identity, col, mesh);
    }
}

}  // namespace

extern "C" int fr_step_convert(const char* path_utf8, double linear_deflection, double angular_deflection,
                               const FrStepCallbacks* cb, double* unit_in_meters, char* error, int error_len) {
    auto fail = [&](const std::string& msg) {
        if (error && error_len > 0) {
            std::strncpy(error, msg.c_str(), (size_t)error_len - 1);
            error[error_len - 1] = '\0';
        }
        return 1;
    };
    try {
        Handle(XCAFApp_Application) app = XCAFApp_Application::GetApplication();
        Handle(TDocStd_Document) doc;
        app->NewDocument("MDTV-XCAF", doc);

        STEPCAFControl_Reader reader;
        reader.SetColorMode(Standard_True);
        reader.SetNameMode(Standard_True);
        reader.SetLayerMode(Standard_False);
        if (cb->progress) cb->progress(cb->user, "lettura del file STEP");
        if (reader.ReadFile(path_utf8) != IFSelect_RetDone) return fail("file STEP non leggibile o non valido");
        if (cb->progress) cb->progress(cb->user, "trasferimento della geometria");
        if (!reader.Transfer(doc)) return fail("trasferimento STEP fallito");

        double unit = 0.001;
        XCAFDoc_DocumentTool::GetLengthUnit(doc, unit);
        if (unit_in_meters) *unit_in_meters = unit;

        Ctx c;
        c.cb = cb;
        c.shapes = XCAFDoc_DocumentTool::ShapeTool(doc->Main());
        c.colors = XCAFDoc_DocumentTool::ColorTool(doc->Main());
        c.lin = linear_deflection;
        c.ang = angular_deflection;

        TDF_LabelSequence roots;
        c.shapes->GetFreeShapes(roots);
        if (roots.Length() == 0) return fail("il file non contiene forme");
        if (cb->progress) cb->progress(cb->user, "triangolazione");
        for (int i = 1; i <= roots.Length(); ++i) {
            const TDF_Label root = roots.Value(i);
            std::string name = label_name(root);
            if (name.empty()) name = "radice-" + std::to_string(i);
            Quantity_Color col;
            const bool has_col = label_color(c, root, col);
            const int node = emit_node(c, -1, name, XCAFDoc_ShapeTool::GetLocation(root), has_col ? &col : nullptr, -1);
            visit_proto(c, root, node, name);
        }
        app->Close(doc);
        return 0;
    } catch (const Standard_Failure& e) {
        return fail(std::string("errore Open CASCADE: ") + (e.GetMessageString() ? e.GetMessageString() : "sconosciuto"));
    } catch (const std::exception& e) {
        return fail(std::string("errore: ") + e.what());
    } catch (...) {
        return fail("errore sconosciuto durante la conversione STEP");
    }
}

// ---------------------------------------------------------------------------
// Generatore di un assieme STEP di prova (per i test automatici): una piastra
// e due istanze della stessa vite, con nomi e colori, esportati con il writer
// STEP ufficiale di OCCT (AP214, unità millimetri).
// ---------------------------------------------------------------------------

#include <BRepPrimAPI_MakeBox.hxx>
#include <BRepPrimAPI_MakeCylinder.hxx>
#include <STEPCAFControl_Writer.hxx>
#include <gp_Ax1.hxx>

extern "C" int fr_step_write_test_assembly(const char* path_utf8, char* error, int error_len) {
    try {
        Handle(XCAFApp_Application) app = XCAFApp_Application::GetApplication();
        Handle(TDocStd_Document) doc;
        app->NewDocument("MDTV-XCAF", doc);
        Handle(XCAFDoc_ShapeTool) st = XCAFDoc_DocumentTool::ShapeTool(doc->Main());
        Handle(XCAFDoc_ColorTool) ct = XCAFDoc_DocumentTool::ColorTool(doc->Main());

        TDF_Label plate = st->AddShape(BRepPrimAPI_MakeBox(100.0, 10.0, 60.0).Shape(), Standard_False);
        TDataStd_Name::Set(plate, "Piastra");
        ct->SetColor(plate, Quantity_Color(0.1, 0.2, 0.8, Quantity_TOC_RGB), XCAFDoc_ColorSurf);

        TDF_Label screw = st->AddShape(BRepPrimAPI_MakeCylinder(4.0, 40.0).Shape(), Standard_False);
        TDataStd_Name::Set(screw, "Vite M8");
        ct->SetColor(screw, Quantity_Color(0.8, 0.1, 0.1, Quantity_TOC_RGB), XCAFDoc_ColorSurf);

        TDF_Label asm_ = st->NewShape();
        TDataStd_Name::Set(asm_, "Staffa");
        TDF_Label c1 = st->AddComponent(asm_, plate, TopLoc_Location());
        TDataStd_Name::Set(c1, "Piastra-1");
        gp_Trsf t1;
        t1.SetTranslation(gp_Vec(20.0, 10.0, 30.0));
        TDF_Label c2 = st->AddComponent(asm_, screw, TopLoc_Location(t1));
        TDataStd_Name::Set(c2, "Vite M8-1");
        gp_Trsf t2;
        t2.SetRotation(gp_Ax1(gp_Pnt(0, 0, 0), gp_Dir(1, 0, 0)), 1.5707963267948966);
        t2.SetTranslationPart(gp_Vec(80.0, 10.0, 30.0));
        TDF_Label c3 = st->AddComponent(asm_, screw, TopLoc_Location(t2));
        TDataStd_Name::Set(c3, "Vite M8-2");
        st->UpdateAssemblies();

        {
            STEPCAFControl_Writer writer;
            writer.SetColorMode(Standard_True);
            writer.SetNameMode(Standard_True);
            if (!writer.Transfer(doc, STEPControl_AsIs)) {
                std::strncpy(error, "trasferimento al writer STEP fallito", (size_t)error_len - 1);
                return 1;
            }
            if (writer.Write(path_utf8) != IFSelect_RetDone) {
                std::strncpy(error, "scrittura STEP fallita", (size_t)error_len - 1);
                return 1;
            }
        }
        // Marcatori su stderr: se il processo va in crash, il test mostra fin dove è arrivato.
        std::fputs("[shim] writer chiuso\n", stderr);
        std::fflush(stderr);
        app->Close(doc);
        std::fputs("[shim] documento chiuso\n", stderr);
        std::fflush(stderr);
        return 0;
    } catch (const Standard_Failure& e) {
        std::strncpy(error, e.GetMessageString() ? e.GetMessageString() : "errore OCCT", (size_t)error_len - 1);
        return 1;
    } catch (...) {
        std::strncpy(error, "errore sconosciuto", (size_t)error_len - 1);
        return 1;
    }
}
