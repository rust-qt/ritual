#!/usr/bin/env python
from BeautifulSoup import BeautifulSoup, NavigableString
import re
import pprint
import glob
import sys
import json
import pprint

pp = pprint.PrettyPrinter()




def fix_nested_types(class_name, t, nested_types):
  def is_nested(name):
    for t in nested_types:
      if t["name"] == name:
        return True
    return False

  if is_nested(t["base"]):
    t["base"] = class_name + "::" + t["base"]

def strip_tags(soup):
  return u' '.join(soup.findAll(text=True))


def parse_type(string):
  string = string.strip()
  if not string:
    raise Exception("Type is missing")
  result = {}
  if "<T>" in string:
    result["template"] = True
  if string.endswith("*"):
    result["pointer"] = True
    string = string[:-1].strip() # no, it's not a smile
  elif string.endswith("&&"):
    print "Double reference type encountered!"
    return False
  elif string.endswith("&"):
    result["reference"] = True
    string = string[:-1].strip()
  if string.startswith("const "):
    result["const"] = True
    string = string[len("const "):].strip()
  result["base"] = string
  if result["base"] == "T":
    result["template"] = True
  return result



def parse_argument(string):
  # dirty hacks section!
  if string in ["int", "bool"]:
    return { "type": parse_type(string), "name": "value" }

  # end of dirty hacks section

  if string == "...":
    return { "variable_arguments": True }
#  print "test ", string
  result = {}
  parts1 = string.split('=')
  if len(parts1) != 1 and len(parts1) != 2: raise Exception("Invalid argument syntax")
  if len(parts1) == 2:
    result["default_value"] = parts1[1].strip()
  other_part = parts1[0].strip()
  re1 = re.findall("\\w+$", other_part)
  if len(re1) != 1: raise Exception("Invalid argument syntax")
  result["name"] = re1[0]
  type_string = other_part[0:len(other_part)-len(result["name"])]
  result["type"] = parse_type(type_string)
  if not result["type"]:
    return False
  return result


def parse_methods(table, class_name, section_attrs, nested_types):
  result = []
  for row in table.findAll("tr"):
    tds = row.findAll("td")
    if len(tds) != 2: raise Exception("Unknown HTML layout")
    return_type_string = tds[0].text.strip()
    signature_string = strip_tags(tds[1]).strip()
    data = section_attrs.copy()
    if class_name:
      data["scope"] = "class"
    else:
      data["scope"] = "global"
    if return_type_string == "typedef":
      continue
    if return_type_string.startswith("virtual"):
      data["virtual"] = True
      return_type_string = return_type_string[len("virtual"):].strip()
    if return_type_string:
      data["return_type"] = parse_type(return_type_string)
      if not data["return_type"]:
        print "Unsupported type encountered. Method is skipped:\n%s\n" % signature_string
        continue
      if class_name:
        fix_nested_types(class_name, data["return_type"], nested_types)
    if re.match("^\\w+$", signature_string):
      data["name"] = signature_string
      data["variable"] = True
      data["type"] = data["return_type"]
      data.pop("return_type")
    else:
      if signature_string.startswith("operator()"):
        name_end_index = signature_string.find("(", len("operator()"))
      else:
        name_end_index = signature_string.find("(")
        if name_end_index < 0: raise Exception("Invalid signature syntax")
      data["name"] = signature_string[0:name_end_index].strip()
      signature_string_parts2 = signature_string[name_end_index+1:].strip().rsplit(")", 1)
      if len(signature_string_parts2) != 2: raise Exception("Invalid signature syntax")
      data["arguments"] = []
      argument_failed = False
      for part in signature_string_parts2[0].strip().split(','):
        if not part: continue
        part = part.strip()
        if part == "...":
          data["variable_arguments"] = True
        else:
          arg = parse_argument(part)
          if not arg:
            argument_failed = True
            break

          if class_name and arg["type"]:
            fix_nested_types(class_name, arg["type"], nested_types)
          data["arguments"].append(arg)
      if argument_failed:
        print "Unsupported type encountered. Method is skipped:\n%s\n" % signature_string
        continue
      if signature_string_parts2[1].strip() == "const":
        data["const"] = True

      if data["name"].startswith("operator"):
        data["operator"] = data["name"][len("operator"):].strip()
      if class_name:
        if data["name"] == class_name:
          data["constructor"] = True
          if "return_type" in data:
            raise Exception("Constructors are not allowed to have return types")
        elif data["name"] == "~" + class_name:
          data["destructor"] = True
          if "return_type" in data:
            raise Exception("Destructors are not allowed to have return types")
        elif "operator" in data:
          pass
          # operators may or may not have return type
        else: # not constructor
          if not "return_type" in data:
            raise Exception("No return type in a method")

    result.append(data)
  return result



def parse_section(soup, class_name, id, attrs, nested_types):
  header = soup.find("h2", id=id)
  if not header: return []
  return parse_methods(header.findNext("table"), class_name, attrs, nested_types)


def parse_macros(soup, id):
  macros_header = soup.find("h2", id=id)
  if not macros_header: return None
  macros = []
  for row in macros_header.findNext("table").findAll("tr"):
    macros.append(row.text)
  return macros


#test1 = {}



def parse_nested_types(soup, class_name_or_namespace):
  all_values = {}
  for table in soup.findAll("table", { "class": "valuelist" }):
    if class_name_or_namespace == "QJsonValue":
      h3 = table.findPrevious("h3")
      if h3 and h3.get("id", "") == "toVariant":
        print "This table is blacklisted!"
        continue

    values = []
    for tr in table.findAll("tr"):
      if len(tr.findAll("th")) > 0: continue # skip header
      tds = tr.findAll("td")
      if not len(tds) in [2, 3]: raise Exception("Unknown HTML layout")
      value = { "name": tds[0].text.strip(), "value": tds[1].text.strip(), "description": strip_tags(tds[2]) if len(tds) > 2 else "" }
      if not value["name"].startswith(class_name_or_namespace + "::"):
        raise Exception("enum item without namespace")
      value["name"] = value["name"][len(class_name_or_namespace + "::"):]
      values.append(value)
    current_pos = table
    found_name = None
    while current_pos:
      current_pos = current_pos.findPrevious("a")
      if current_pos and current_pos.get("name") and current_pos["name"].endswith("-enum"):
        found_name = current_pos["name"]
        break
    if not found_name:
      raise Exception("Can't find anchor for values table!")
    all_values[found_name] = all_values.get(found_name, []) +  values

  all_types = []
  link_href_to_enum = {}

  types_header = soup.find("h2", { "id": "Typesx" })
  if not types_header:
    types_header = soup.find("h2", { "id": "public-types" })
  if not types_header:
    types_header = soup.find("h2", { "id": "types" })
  if not types_header:
    print "no types header"
    return []
  for row in types_header.findNext("table").findAll("tr"):
    tds = row.findAll("td")
    if len(tds) != 2: raise Exception("Unknown HTML layout")
    kind_of_type = tds[0].text
    if kind_of_type == "enum":
      link = tds[1].find("a")
      if not link: raise Exception("enum without link")
      if not link.get("href"): raise Exception("enum link without href")
      if not "#" in link["href"]: raise Exception("enum link href without anchor")
      anchor = link["href"].split("#")[1]
      name = tds[1].text.split("{")[0].strip()
      if not anchor in all_values:
        raise Exception("values table not found for enum")
      all_types.append({
        "kind": "enum",
        "name": name,
        "values": all_values[anchor]})
      link_href_to_enum[link["href"]] = name

  for row in types_header.findNext("table").findAll("tr"):
    tds = row.findAll("td")
    if len(tds) != 2: raise Exception("Unknown HTML layout")
    kind_of_type = tds[0].text
    if kind_of_type == "enum": continue
    name = tds[1].text.strip()
    if kind_of_type == "flags":
      link = tds[1].find("a")
      if not link: raise Exception("flags without link")
      if not link.get("href"): raise Exception("flags link without href")
      if not link["href"] in link_href_to_enum:
        raise Exception("no enum found for flags")
      enum = link_href_to_enum[link["href"]]
      all_types.append({
        "kind": "flags",
        "name": name,
        "enum": enum})
    else:
      all_types.append({
        "kind": kind_of_type,
        "name": name})

  if class_name_or_namespace == "QByteArray":
    # they forgot to document this!
    all_types.append({ "kind": "typedef", "name": "iterator" })
    all_types.append({ "kind": "typedef", "name": "const_iterator" })



  return all_types








def parse_doc(filename):
  print "Parsing " + filename
  data = open(filename,'r').read()
  soup = BeautifulSoup(data, convertEntities=BeautifulSoup.HTML_ENTITIES)
  result = {}
  title = soup.find("h1", { "class": "title" })
  if not title: raise Exception("Unknown HTML layout")


  if title.text.startswith("<Qt") or title.text == "Qt Namespace":
    result["type"] = "global"
    macros = parse_macros(soup, "Macrosx")
    if macros: result["macros"] = macros
    if title.text == "Qt Namespace":
      result["include_file"] = "Qt"
      result["nested_types"] = parse_nested_types(soup, "Qt")
    else:
      end_index = title.text.find(">")
      if not end_index:
        raise Exception("Invalid global header title")
      result["include_file"] = title.text[1:end_index]

    result["methods"] = parse_section(soup, None, "Functions", {}, result.get("nested_types", []))
    return result

  title_parts = title.text.split(' ')
  if len(title_parts) == 2 and title_parts[1].strip() == "Class":
    result["type"] = "class"
    class_without_namespace = title_parts[0].strip()
    result["include_file"] = class_without_namespace
    full_class_tag = soup.find("span", { "class": "small-subtitle" })
    if full_class_tag:
      result["class"] = full_class_tag.text.strip(" ()")
    else:
      result["class"] = class_without_namespace
    result["nested_types"] = parse_nested_types(soup, result["class"])

    if not result["class"].startswith("Q"):
      print "Warning: %s: class %s doesn't start with Q" % (filename, result["class"])
  else:
    print "Unsupported title value in %s" % filename
    return None


  result["methods"] = []

  for section_name, attrs in [
    ("public-functions", {}),
    ("protected-functions", { "protected": True }),
    ("public-slots", { "slot": True }),
    ("protected-slots", { "slot": True, "protected": True }),
    ("static-public-members", { "static": True }),
    ("static-protected-members", { "static": True, "protected": True }),
    ("signals", { "signal": True }),
    ("related-non-members", {})
  ]:
    cl = None if section_name == "related-non-members" else class_without_namespace
    result["methods"] += parse_section(soup, cl, section_name, attrs, result["nested_types"])
  macros = parse_macros(soup, "macros")
  if macros: result["macros"] = macros

  return result

#pp = pprint.PrettyPrinter()


def parse(input_folder):
  all_data = []
  #for filename in ['../qt-doc/html/qtendian.html']:
  for filename in glob.iglob(input_folder + '/*.html'):
    bad_endings = "members", "obsolete", "compat", "example", "pro", "cpp", "h", "ui"
    bad = False
    for ending in bad_endings:
      if filename.endswith("-%s.html" % ending):
        bad = True
        break
    if bad: continue

    #print filename
    result = parse_doc(filename)
    if result:
      all_data.append(result)
    #pp.pprint(result)
  return all_data

if len(sys.argv) < 3:
  print "Usage: parser doc_html_folder output_filename"
else:
  print "Parsing documentation..."
  parse_result = parse(sys.argv[1])
  with open(sys.argv[2], "w") as f:
    json.dump(parse_result, f, indent=2)

  print "Done."

  #print test1

